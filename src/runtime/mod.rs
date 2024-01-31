//! 运行时环境
//! 将解析的ast放在实际作用域中运行

use crate::ast::*;
use crate::intern::{intern, Interned};
use std::collections::HashMap;
use std::mem::transmute;
use std::sync::atomic::{AtomicUsize,self};
use std::ptr::NonNull;
use std::ops::{Deref,DerefMut};

mod gc;
pub use gc::LocalFunc;
mod io;


/// 运行期追踪行号
/// 
/// 只有主线程会访问，不存在多线程同步问题
static mut LINE:usize = 0;
pub fn err(s:&str)-> ! {
  panic!("{} 运行时({})", s, unsafe{LINE})
}

#[derive(Debug)]
pub struct Module {
  pub imports: Vec<ModDef>,
  pub export: ModDef
}


/// 一个运行时作用域
/// 
/// run函数需要mut因为需要跟踪行数
/// 
/// return_to是用来标志一个函数是否返回过了。
/// 如果没返回，Some()里就是返回值要写入的指针
#[derive(Debug)]
pub struct ScopeInner {
  /// 父作用域
  parent: Option<Scope>,
  /// 返回值指针
  return_to: *mut Option<*mut Litr>,
  /// (类型名,值)
  structs: Vec<(Interned, KsType)>,
  /// (变量名,值)
  vars: Vec<(Interned, Litr)>,
  /// 导入和导出的模块指针
  mods: *mut Module,
  /// 引用计数
  count: AtomicUsize
}


/// 作用域指针
#[derive(Debug, Clone, Copy)]
pub struct Scope {
  pub p:NonNull<ScopeInner>
}
impl Scope {
  pub fn new(s:ScopeInner)-> Self {
    Scope {
      p: NonNull::new(Box::into_raw(Box::new(s))).unwrap()
    }
  }
  pub fn uninit()-> Self {
    Scope {p: NonNull::dangling()}
  }
}
impl Deref for Scope {
  type Target = ScopeInner;
  fn deref(&self) -> &Self::Target {
    unsafe {self.p.as_ref()}
  }
}
impl DerefMut for Scope {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {self.p.as_mut()}
  }
}

impl Scope {
  /// 在此作用域运行ast代码
  pub fn run(&mut self, codes:&Statements) {
    for (l, sm) in &codes.0 {
      unsafe{LINE = *l;}

      // 如果子作用域返回过了，这里就会是Returned状态
      let return_to = unsafe{&*self.return_to};
      if let None = return_to {
        return;
      }

      // 遇到return语句就停止当前遍历
      if let Stmt::Return(expr) = sm {
        unsafe {
          if let Some(p) = return_to {
            **p = self.calc(expr);
          }
          *self.return_to = None;
        }
        return;
      }

      self.evil(sm);
    }
  }

  /// 在作用域解析一个语句
  pub fn evil(&mut self, code:&Stmt) {
    use Stmt::*;
    match code {
      Expression(e)=> {
        self.calc(e);
      }
      Let(a)=> {
        let mut v = self.calc(&a.val);
        // 不检查变量是否存在是因为寻找变量的行为是反向的
        self.vars.push((a.id, v));
      }
      Block(s)=> {
        let mut scope = Scope::new(ScopeInner {
          parent:Some(*self),
          return_to: self.return_to,
          structs:Vec::new(),
          vars: Vec::with_capacity(16),
          mods: self.mods,
          count: AtomicUsize::new(0)
        });
        scope.run(s);
      }
      Mod(m)=> {
        unsafe {
          (*self.mods).imports.push((**m).clone());
        }
      }
      Export(e)=> {
        match &**e {
          ExportDef::Func((id, f)) => {
            let mut f = f.clone();
            f.scope = *self;
            let fp = LocalFunc::new(f);
            // 导出函数则必须多增加一层引用计数，保证整个程序期间都不会被释放
            fp.count_enc();
            let exec = Executable::Local(fp);
            self.vars.push((*id, Litr::Func(Box::new(exec.clone()))));
            unsafe{(*self.mods).export.funcs.push((*id,exec))}
          }
        }
      }
      Return(_)=> err("return语句不应被直接evil"),
      _=> {}
    }
  }

  /// 调用一个函数
  pub fn call(&mut self, call: &Box<CallDecl>)-> Litr {
    let targ = self.calc(&call.targ);

    // 将参数解析为参数列表
    let arg = self.calc(&call.args);
    let mut args = Vec::new();
    if let Litr::Array(l) = arg {
      args = *l;
    }else {
      args.push(arg);
    }
    if let Litr::Func(exec) = targ {
      use Executable::*;
      match *exec {
        Native(f)=> f(args),
        Local(f)=> self.call_local(&f, args),
        Extern(f)=> self.call_extern(&f, args)
      }
    }
    else {err(&format!("'{:?}' 不是一个函数", targ))}
  }

  /// 调用本地定义的函数
  pub fn call_local(&mut self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    // 将传入参数按定义参数数量放入作用域
    let mut vars = Vec::with_capacity(16);
    let mut args = args.into_iter();
    for  (name,ty) in f.argdecl.iter() {
      let arg = args.next().unwrap_or(Litr::Uninit);
      vars.push((*name,arg))
    }

    let mut ret = Litr::Uninit;
    let mut return_to = Some(&mut ret as *mut Litr);
    let mut scope = Scope::new(ScopeInner {
      parent:Some(f.scope),
      return_to:&mut return_to,
      structs:Vec::new(),
      vars,
      mods: self.mods,
      count: AtomicUsize::new(0)
    });
    scope.run(&f.exec);
    ret
  }

  /// 调用extern函数
  pub fn call_extern(&mut self, f:&ExternFunc, args:Vec<Litr>)-> Litr {
    use crate::extern_agent::translate;
    let len = f.argdecl.len();
    let mut args = args.into_iter();

    macro_rules! impl_arg {
      {$(
        $n:literal $($arg:ident)*
      )*} => {
        match len {
          $(
            $n => {
              let callable:extern fn($($arg:usize,)*)-> usize = unsafe {transmute(f.ptr)};
              let mut eargs = [0usize;$n];
              eargs.iter_mut().enumerate().for_each(|(i,p)| {
                if let Some(v) = args.next() {
                  let transed = translate(v).unwrap_or_else(|e|err(&e));
                  *p = transed
                }
              });
              let [$($arg,)*] = eargs;
              let ret = callable($($arg,)*);
              Litr::Uint(ret)
            }
          )*
          _=> {err(&format!("extern函数不支持{}位参数", len))}
        }
      }
    }
    impl_arg!{
      0
      1  a
      2  a b
      3  a b c
      4  a b c d
      5  a b c d e 
      6  a b c d e f 
      7  a b c d e f g
      8  a b c d e f g h
      9  a b c d e f g h i 
      10 a b c d e f g h i j
      11 a b c d e f g h i j k
      12 a b c d e f g h i j k l
      13 a b c d e f g h i j k l m
      14 a b c d e f g h i j k l m n
      15 a b c d e f g h i j k l m n o
    }
  }

  /// 在作用域找一个变量
  pub fn var(&mut self, s:Interned)-> &mut Litr {
    let inner = &mut (**self);
    for (p, v) in inner.vars.iter_mut().rev() {
      if *p == s {
        return v;
      }
    }

    if let Some(parent) = &mut inner.parent {
      return parent.var(s);
    }
    err(&format!("无法找到变量 '{}'", s.str()));
  }


  /// 在此作用域计算表达式的值
  /// 会将变量计算成实际值
  pub fn calc(&mut self, e:&Expr)-> Litr {
    use Litr::*;
    match e {
      Expr::Call(c)=> self.call(c),

      Expr::Literal(litr)=> {
        let ret = if let Variant(id) = litr {
          self.var(*id).clone()
        }else {
          litr.clone()
        };
        return ret;
      }

      Expr::LocalDecl(local)=> {
        let mut f = (**local).clone();
        f.scope = *self;
        let exec = Executable::Local(LocalFunc::new(f));
        Litr::Func(Box::new(exec))
      }

      Expr::Binary(bin)=> {
        /// 二元运算中普通数字的戏份
        macro_rules! impl_num {
          ($pan:literal $op:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            impl_num!($pan,left,right $op)
          }};
          ($pan:literal $op:tt $n:tt)=> {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if match right {
              Int(r) => r == 0,
              Uint(r) => r == 0,
              Float(r) => r == 0.0,
              _=> false
            } {err("除数必须非0")}
            impl_num!($pan,left,right $op)
          }};
          ($pan:literal,$l:ident,$r:ident $op:tt) => {{
            match ($l, $r) {
              (Int(l),Int(r))=> Int(l $op r),
              (Uint(l),Uint(r))=> Uint(l $op r),
              (Uint(l),Int(r))=> Uint(l $op r as usize),
              (Float(l),Float(r))=> Float(l $op r),
              (Float(l),Int(r))=> Float(l $op r as f64),
              _=> err($pan)
            }
          }};
        }

        /// 二元运算中无符号数的戏份
        macro_rules! impl_unsigned {
          ($pan:literal $op:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            match (left, right) {
              (Uint(l), Uint(r))=> Uint(l $op r),
              (Uint(l), Int(r))=> Uint(l $op r as usize),
              _=> err($pan)
            }
          }};
        }

        /// 数字修改并赋值
        macro_rules! impl_num_assign {
          ($o:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if let Expr::Literal(Variant(id)) = bin.left {
              // 将Int自动转为对应类型
              let n = match (left, right) {
                (Uint(l), Uint(r))=> Uint(l $o r),
                (Uint(l), Int(r))=> Uint(l $o r as usize),
                (Int(l), Int(r))=> Int(l $o r),
                (Float(l), Float(r))=> Float(l $o r),
                (Float(l), Int(r))=> Float(l $o r as f64),
                _=> panic!("运算并赋值的左右类型不同 运行时({})", unsafe{LINE})
              };
              *self.var(id) = n;
              return Uninit;
            }
            err("只能为变量赋值。");
          }};
        }

        // 无符号数修改并赋值
        macro_rules! impl_unsigned_assign {
          ($op:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if let Expr::Literal(Variant(id)) = bin.left {
              let line = unsafe{LINE};
              // 数字默认为Int，所以所有数字类型安置Int自动转换
              let n = match (left, right) {
                (Uint(l), Uint(r))=> Uint(l $op r),
                (Uint(l), Int(r))=> Uint(l $op r as usize),
                _=> panic!("按位运算并赋值只允许无符号数 运行时({})", line)
              };
              *self.var(id) = n;
              return Uninit;
            }
            err("只能为变量赋值。");
          }};
        }

        /// 比大小宏
        /// 
        /// 故意要求传入left和right是为了列表比较时能拿到更新后的left right
        macro_rules! impl_ord {
          ($o:tt) => {{
            let mut left = self.calc(&bin.left);
            let mut right = self.calc(&bin.right);
            impl_ord!($o,left,right)
          }};
          ($o:tt,$l:expr,$r:expr) => {{
            // 将整数同化为Int
            if let Uint(n) = $l {
              $l = Int(n as isize)
            }
            if let Uint(n) = $r {
              $r = Int(n as isize)
            }

            let b = match ($l, $r) {
              (Uint(l),Uint(r))=> l $o r,
              (Int(l), Int(r))=> l $o r,
              (Float(l), Float(r))=> l $o r,
              (Int(l), Float(r))=> (l as f64) $o r,
              (Float(l), Int(r))=> l $o r as f64,
              (Str(l), Str(r))=> l $o r,
              (Bool(l), Bool(r))=> l $o r,
              (Buffer(l), Buffer(r))=> {
                use Buf::*;
                match ( *l,*r ) {
                  (U8(l),U8(r))=> l $o r,
                  (U16(l),U16(r))=> l $o r,
                  (U32(l),U32(r))=> l $o r,
                  (U64(l),U64(r))=> l $o r,
                  (I8(l),I8(r))=> l $o r,
                  (I16(l),I16(r))=> l $o r,
                  (I32(l),I32(r))=> l $o r,
                  (I64(l),I64(r))=> l $o r,
                  (F32(l),F32(r))=> l $o r,
                  (F64(l),F64(r))=> l $o r,
                  _=> false
                }
              },
              _=> false
            };
            b
          }};
        }

        /// 逻辑符
        macro_rules! impl_logic {
          ($o:tt) => {{
            let mut left = self.calc(&bin.left);
            let mut right = self.calc(&bin.right);
            // 先把uninit同化成false
            if let Uninit = left {
              left = Bool(false)
            }
            if let Uninit = right {
              right = Bool(false)
            }

            match (left, right) {
              (Bool(l), Bool(r))=> Bool(l $o r),
              _=> err("逻辑运算符两边必须都为Bool")
            }
          }};
        }

        match &*bin.op {
          // 数字
          b"+" => {
            // 字符加法
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if let Str(mut s) = left {
              let r = right.str();
              s.push_str(&r);
              return Str(s);
            }
            impl_num!("相加类型不同",left,right +)
          },
          b"-" => impl_num!("相减类型不同" -),
          b"*" => impl_num!("相乘类型不同" *),
          b"%" => impl_num!("求余类型不同" %),
          b"/" => impl_num!("相除类型不同" / 0),

          // unsigned
          b"<<" => impl_unsigned!("左移需要左值无符号" <<),
          b">>" => impl_unsigned!("右移需要左值无符号" >>),
          b"&" => impl_unsigned!("&需要左值无符号" &),
          b"^" => impl_unsigned!("^需要左值无符号" ^),
          b"|" => impl_unsigned!("|需要左值无符号" |),

          // 赋值
          b"=" => {
            let right = self.calc(&bin.right);
            if let Expr::Literal(Variant(id)) = bin.left {
              *self.var(id) = right;
            }
            return Uninit;
          }
          b"+=" => impl_num_assign!(+),
          b"-=" => impl_num_assign!(-),
          b"*=" => impl_num_assign!(*),
          b"/=" => impl_num_assign!(/),
          b"%=" => impl_num_assign!(%),

          b"&=" => impl_unsigned_assign!(&),
          b"^=" => impl_unsigned_assign!(^),
          b"|=" => impl_unsigned_assign!(|),
          b"<<=" => impl_unsigned_assign!(<<),
          b">>=" => impl_unsigned_assign!(>>),

          // 比较
          b"==" => {
            let mut left = self.calc(&bin.left);
            let mut right = self.calc(&bin.right);
            // 列表类型只能用==比较
            if let Array(l) = left {
              if let Array(r) = right {
                let b = l.into_iter().zip(r.into_iter()).all(|(mut l,mut r)| {
                  impl_ord!(==,l,r)
                });
                Bool(b)
              }else {
                Bool(false)
              }
            }
            else {
              Bool(impl_ord!(==,left,right))
            }
          }
          b"!=" => Bool(impl_ord!(!=)),
          b">=" => Bool(impl_ord!(>=)),
          b"<=" => Bool(impl_ord!(<=)),
          b">" => Bool(impl_ord!(>)),
          b"<" => Bool(impl_ord!(<)),

          // 逻辑
          b"&&" => impl_logic!(&&),
          b"||" => impl_logic!(||),

          // 解析,运算符
          b"," => {
            let left = self.calc(&bin.left);
            // 允许空右值
            if let Expr::Empty = bin.right {
              if let Array(_) = left {
                return left;
              }else {
                return Array(Box::new(vec![left]));
              }
            }
            // 有右值的情况
            let right = self.calc(&bin.right);
            if let Array(mut o) = left {
              o.push(right);
              Array(o)
            }else {
              Array(Box::new(vec![left, right]))
            }
          }
          _=> err(&format!("未知运算符'{}'", String::from_utf8_lossy(&bin.op)))
        }
      }

      Expr::Unary(una)=> {
        let right = self.calc(&una.right);
        match una.op {
          b'-'=> {
            match right {
              Int(n)=> Int(-n),
              Float(n)=> Float(-n),
              _=> err("负号只能用在有符号数")
            }
          }
          b'!'=> {
            match right {
              Bool(b)=> Bool(!b),
              Int(n)=> Int(!n),
              Uint(n)=> Uint(!n),
              Uninit => Bool(true),
              _=> err("!运算符只能用于整数和Bool")
            }
          }_=>Uninit
        }
      }

      Expr::Buffer(decl)=> {
        let expr = &decl.expr;
        // Buffer是由Array构造的
        let vec = {
          if let Expr::Empty = expr {
            Vec::new()
          }else {
            let l = self.calc(expr);
            if let Array(vptr) = l {
              *vptr
            }else {
              vec![l]
            }
          }
        };

        use Buf::*;
        /// 匹配Buffer类型
        macro_rules! impl_num {($t:ty,$e:expr) => {{
          let mut v = Vec::<$t>::new();
          for l in vec.into_iter() {
            match l {
              Int(n)=> v.push(n as $t),
              Uint(n)=> v.push(n as $t),
              Float(n)=> v.push(n as $t),
              _=> v.push(0.0 as $t)
            }
          }
          return Buffer(Box::new($e(v)));
        }}}

        let ty = &*decl.ty;
        match ty {
          b"u8"=> impl_num!(u8,U8),
          b"u16"=> impl_num!(u16,U16),
          b"u32"=> impl_num!(u32,U32),
          b"u64"=> impl_num!(u64,U64),
          b"i8"=> impl_num!(i8,I8),
          b"i16"=> impl_num!(i16,I16),
          b"i32"=> impl_num!(i32,I32),
          b"i64"=> impl_num!(i64,I64),
          b"f32"=> impl_num!(f32,F32),
          b"f64"=> impl_num!(f64,F64),
          _=> err("未知的Buffer类型")
        }
      }

      Expr::ModFuncAcc(acc)=> {
        let modname = acc.left;
        let funcname = acc.right;
        unsafe {
          for def in (*self.mods).imports.iter() {
            if def.name == modname {
              for (id, func) in def.funcs.iter() {
                if *id == funcname {
                  // 模块导出的函数必定不能回收
                  // 因此使用模块函数不需要考虑其回收
                  return Litr::Func(Box::new(func.clone()));
                }
              }
              err(&format!("模块'{}'中没有'{}'函数",modname,funcname))
            }
          }
          err(&format!("当前作用域没有'{}'模块",modname))
        }
      }

      Expr::Empty => err("得到空表达式"),
      _=> err("算不出来 ")
    }
  }

}


#[derive(Debug)]
pub struct RunResult {
  pub returned: Litr,
  pub exported: ModDef
}

/// 创建顶级作用域并运行一段程序
pub fn run(s:&Statements)-> RunResult {
  let mut top_ret = Litr::Uint(0);
  let mut return_to = &mut Some(&mut top_ret as *mut Litr);
  let mut mods = Module { 
    imports: Vec::new(), 
    export: ModDef { name: intern(b"mod"), funcs: Vec::new() } 
  };
  top_scope(return_to, &mut mods).run(s);
  RunResult { returned: top_ret, exported: mods.export }
}

/// 创建顶级作用域
/// 
/// 自定义此函数可添加初始函数和变量
pub fn top_scope(return_to:*mut Option<*mut Litr>, mods:*mut Module)-> Scope {
  let mut vars = Vec::<(Interned, Litr)>::with_capacity(16);
  vars.push((intern(b"print"), 
    Litr::Func(Box::new(Executable::Native(io::print))))
  );
  Scope::new(ScopeInner {
    parent: None, 
    return_to, 
    structs:Vec::new(), 
    vars, mods, 
    count: AtomicUsize::new(0)
  })
}

