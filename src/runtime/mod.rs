//! 运行时环境
//! 将解析的ast放在实际作用域中运行

use crate::ast::*;
use crate::intern::{intern, Interned};
use crate::allocated::leak;
use std::collections::HashMap;
use std::f32::consts::E;
use std::mem::transmute;

mod io;


/// 一个运行时作用域
/// 
/// run函数需要mut因为需要跟踪行数
/// 
/// return_to是用来标志一个函数是否返回过了。
/// 如果没返回，Some()里就是返回值要写入的指针
#[derive(Debug)]
pub struct Scope { 
  parent: Option<*mut Scope>,
  return_to: *mut Option<*mut Litr>,
  types: Vec<(Interned, KsType)>,
  vars: Vec<(Interned, Litr)>,
  line: usize
}

impl Scope {
  /// 在此作用域运行ast代码
  pub fn run(&mut self, codes:&Statements) {
    for (l, sm) in &codes.exec {
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

      self.line = *l;
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
        let v = self.calc(&a.val);
        self.let_var(a.id, v);
      }
      Block(s)=> {
        let mut scope = Scope {
          parent:Some(self),
          line:0,
          return_to: self.return_to,
          types:Vec::new(),
          vars: Vec::new()
        };
        scope.run(s);
      }
      _=> {}
    }
  }

  /// 调用一个函数
  pub fn call(&mut self, call: &Box<Call>)-> Litr {
    let targ = self.calc(&call.targ);

    // 将参数解析为参数列表
    let arg = self.calc(&call.args);
    let mut args = Vec::new();
    if let Litr::Array(l) = arg {
      args.append(unsafe{&mut *l});
    }else {
      args.push(arg);
    }
    if let Litr::Func(exec) = targ {
      use Executable::*;
      let exec = unsafe {&*exec};
      match exec {
        Runtime(f)=> f(args),
        Local(f)=> self.call_local(f, args),
        Extern(f)=> self.call_extern(f, args)
      }
    }
    else {self.err(&format!("'{:?}' 不是一个函数", targ))}
  }

  /// 调用本地定义的函数
  pub fn call_local(&mut self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    // 将传入参数按定义参数数量放入作用域
    let mut vars = Vec::with_capacity(f.args.len());
    let slots = vars.spare_capacity_mut();
    for (i,(name,_)) in f.args.iter().enumerate() {
      let arg = *args.get(i).unwrap_or(&Litr::Uninit);
      slots[i].write((*name, arg));
    }
    unsafe {vars.set_len(f.args.len());}

    let mut ret = Litr::Uninit;
    let mut return_to = Some(&mut ret as *mut Litr);
    let mut scope = Scope {
      parent:Some(self),
      line:0,
      return_to:&mut return_to,
      types:Vec::new(),
      vars
    };
    scope.evil(&f.exec);
    ret
  }

  /// 调用extern函数
  pub fn call_extern(&mut self, f:&ExternFunc, args:Vec<Litr>)-> Litr {
    use crate::extern_agent::{
      set_scope, translate
    };
    let len = f.args.len();
    let args:Vec<usize> = args.into_iter().map(|v| match translate(v) {
      Ok(v)=> v,
      Err(e)=> self.err(&e)
    }).collect();
    
    set_scope(self);
    macro_rules! impl_arg {
      {$(
        $n:literal $($arg:ident)*
      )*} => {
        match len {
          $(
            $n => {
              let callable:extern fn($($arg:usize,)*)-> usize = unsafe {transmute(f.ptr)};
              let mut v = [0usize;$n];
              v.iter_mut().enumerate().for_each(|(i,p)| {
                if let Some(v) = args.get(i) {
                  *p = *v
                }
              });
              let [$($arg,)*] = v;
              let ret = callable($($arg,)*);
              Litr::Uint(ret)
            }
          )*
          _=> {self.err(&format!("extern函数不支持{}位参数", len))}
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
  pub fn var(&self, s:Interned)-> &Litr {
    for (p, v) in self.vars.iter() {
      if *p == s {
        return v;
      }
    }
    if let Some(parent) = self.parent {
      let d = unsafe {&*parent};
      return d.var(s);
    }
    self.err(&format!("无法找到变量 '{}'", s.str()));
  }

  /// 寻找并修改一个变量
  pub fn modify_var(&mut self, s:Interned, f:impl FnOnce(&mut Litr)) {
    for (id,v) in &mut self.vars {
      if *id == s {
        f(v);
        return;
      }
    }
    if let Some(parent) = self.parent {
      let d = unsafe {&mut *parent};
      d.modify_var(s, f);
    }
    self.err(&format!("无法找到变量 '{}'", s.str()));
  }

  pub fn let_var(&mut self, s:Interned, v:Litr) {
    for (id, exist_v) in &mut self.vars {
      if *id == s {
        *exist_v = v;
        return;
      }
    };
    self.vars.push((s, v));
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
            } {self.err("除数必须非0")}
            impl_num!($pan,left,right $op)
          }};
          ($pan:literal,$l:ident,$r:ident $op:tt) => {{
            match ($l, $r) {
              (Int(l),Int(r))=> Int(l $op r),
              (Uint(l),Uint(r))=> Uint(l $op r),
              (Uint(l),Int(r))=> Uint(l $op r as usize),
              (Float(l),Float(r))=> Float(l $op r),
              (Float(l),Int(r))=> Float(l $op r as f64),
              _=> self.err($pan)
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
              _=> self.err($pan)
            }
          }};
        }

        /// 数字修改并赋值
        macro_rules! impl_num_assign {
          ($o:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if let Expr::Literal(Variant(id)) = bin.left {
              let line = self.line;
              let f = |p: &mut Litr|{
                // 数字默认为Int，所以所有数字类型安置Int自动转换
                let n = match (left, right) {
                  (Uint(l), Uint(r))=> Uint(l $o r),
                  (Uint(l), Int(r))=> Uint(l $o r as usize),
                  (Int(l), Int(r))=> Int(l $o r),
                  (Float(l), Float(r))=> Float(l $o r),
                  (Float(l), Int(r))=> Float(l $o r as f64),
                  _=> panic!("运算并赋值的左右类型不同 运行时({})", line)
                };
                *p = n;
              };
              self.modify_var(id, f);
              return right;
            }
            self.err("只能为变量赋值。");
          }};
        }

        // 无符号数修改并赋值
        macro_rules! impl_unsigned_assign {
          ($op:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if let Expr::Literal(Variant(id)) = bin.left {
              let line = self.line;
              let f = |p: &mut Litr|{
                // 数字默认为Int，所以所有数字类型安置Int自动转换
                let n = match (left, right) {
                  (Uint(l), Uint(r))=> Uint(l $op r),
                  (Uint(l), Int(r))=> Uint(l $op r as usize),
                  _=> panic!("按位运算并赋值 左右类型不同 运行时({})", line)
                };
                *p = n;
              };
              self.modify_var(id, f);
              return right;
            }
            self.err("只能为变量赋值。");
          }};
        }

        /// 比大小宏
        /// 
        /// 故意要求传入left和right是为了列表比较时能拿到更新后的left right
        macro_rules! impl_ord {
          ($o:tt) => {{
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            impl_ord!($o,left,right)
          }};
          ($o:tt,$l:ident,$r:ident) => {{
            // 将整数同化为Int
            let left = match $l {
              Uint(n)=> Int((n) as isize),
              _=> $l
            };
            let right = match $r {
              Uint(n)=> Int((n) as isize),
              _=> $r
            };

            let b = match (left, right) {
              (Uint(l),Uint(r))=> l $o r,
              (Int(l), Int(r))=> l $o r,
              (Float(l), Float(r))=> l $o r,
              (Int(l), Float(r))=> (l as f64) $o r,
              (Float(l), Int(r))=> l $o r as f64,
              (Str(l), Str(r))=> l $o r,
              (Bool(l), Bool(r))=> l $o r,
              (Buffer(l), Buffer(r))=> {
                use Buf::*;
                let (l,r) = unsafe {(&*l,&*r)};
                match ( l,r ) {
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
              _=> self.err("逻辑运算符两边必须都为Bool")
            }
          }};
        }

        match &*bin.op {
          // 数字
          b"+" => {
            // 字符加法
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            if let Str(s) = left {
              let r = right.str();
              unsafe{(*s).push_str(&r);}
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
              self.modify_var(id, |p|*p = right.clone());
            }
            return right;
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
            let left = self.calc(&bin.left);
            let right = self.calc(&bin.right);
            // 列表类型只能用==比较
            match (left,right) {
              (Array(l),Array(r))=> {
                let (l,r) = unsafe{((*l).clone(),(*r).clone())};
                let b = l.iter().copied().zip(r.iter().copied()).all(|(left,right)| {
                  impl_ord!(==,left,right)
                });
                Bool(b)
              }
              _=> Bool(impl_ord!(==,left,right))
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
                return Array(leak(vec![left]));
              }
            }
            // 有右值的情况
            let right = self.calc(&bin.right);
            if let Array(o) = left {
              unsafe { (*o).push(right); }
              Array(o)
            }else {
              Array(leak(vec![left, right]))
            }
          }
          _=> self.err(&format!("未知运算符'{}'", String::from_utf8_lossy(&bin.op)))
        }
      }

      Expr::Unary(una)=> {
        let right = self.calc(&una.right);
        match una.op {
          b'-'=> {
            match right {
              Int(n)=> Int(-n),
              Float(n)=> Float(-n),
              _=> self.err("负号只能用在有符号数")
            }
          }
          b'!'=> {
            match right {
              Bool(b)=> Bool(!b),
              Int(n)=> Int(!n),
              Uint(n)=> Uint(!n),
              Uninit => Bool(true),
              _=> self.err("!运算符只能用于整数和Bool")
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
              unsafe {(*vptr).clone()}
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
          return Buffer(leak($e(v)));
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
          _=> self.err("未知的Buffer类型")
        }
      }
      Expr::Empty => self.err("得到空表达式"),
      _=> self.err("算不出来 ")
    }
  }

  pub fn err(&self, s:&str)-> ! {
    panic!("{} 运行时({})", s, self.line)
  }
}



/// 创建顶级作用域并运行一段程序
pub fn run(s:&Statements)-> Litr {
  let mut top_ret = Litr::Uint(0);
  let mut return_to = Some(&mut top_ret as *mut Litr);
  top_scope(&mut return_to).run(s);
  top_ret
}

/// 创建顶级作用域
/// 
/// 自定义此函数可添加初始函数和变量
pub fn top_scope(return_to:*mut Option<*mut Litr>)-> Scope {
  let types = Vec::<(Interned, KsType)>::new();
  let mut vars = Vec::<(Interned, Litr)>::new();
  vars.push((intern(b"print"), 
    Litr::Func(leak(Executable::Runtime(io::print))))
  );
  Scope {parent: None, return_to, types, vars, line:0}
}

