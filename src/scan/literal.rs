use super::{
  charts, intern, Scanner,
  stmt::{Statements, ClassDef},
  expr::*
};

use crate::{
  native::NativeInstance, 
  runtime::{calc::CalcRef, Module, Scope}
};
use crate::intern::Interned;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Function), 
  Str    (String),
  Buffer (Vec<u8>),
  List   (Vec<Litr>),
  Obj    (HashMap<Interned, Litr>),
  Inst   (Instance),
  Ninst  (NativeInstance),
  Sym    (crate::primitive::sym::Symbol)
}
impl Litr {
  /// 由Key编译器提供的转字符
  pub fn str(&self)-> String {
    use Litr::*;
    match self {
      Uninit => String::default(),
      Int(n)=> n.to_string(),
      Uint(n)=> n.to_string(),
      Float(n)=> n.to_string(),
      Bool(n)=> n.to_string(),
      Func(f)=> {
        match *f {
          Function::Local(_)=> "<Local Function>".to_string(),
          Function::Extern(_)=> "<Extern Function>".to_string(),
          Function::Native(_)=> "<Native Function>".to_string(),
          Function::NativeMethod(_)=> "<Native Method>".to_string()
        }
      }
      Str(s)=> s.clone(),
      List(a) => {
        let mut iter = a.iter();
        let mut str = String::new();
        str.push_str("[");
        if let Some(v) = iter.next() {
          str.push_str(&v.str());
        };
        while let Some(v) = iter.next() {
          str.push_str(", ");
          str.push_str(&v.str());
        }
        str.push_str("]");
        str
      },
      Buffer(b)=> format!("{:?}",b),
      Obj(map)=> {
        let mut s = String::new();
        s.push_str("{ ");
        let mut itr = map.iter();
        macro_rules! next {($k:ident,$v:ident)=>{{
          s.push_str(&$k.str());
          let v = $v.str();
          if v != "" {
            s.push_str(": ");
            s.push_str(&v);
          }
        }}}
        if let Some((k,v)) = itr.next() {
          next!(k,v)
        }
        while let Some((k, v)) = itr.next() {
          s.push_str(", ");
          next!(k,v)
        }
        s.push_str(" }");
        s
      },
      Inst(i)=> {
        let cls = unsafe{&*i.cls};
        let mut name = cls.props.iter();
        let mut val = i.v.iter();
        let mut s = String::new();
        macro_rules! next {($p:ident) => {{
          s.push_str(&$p.name.str());
          let next_v = val.next().unwrap().str();
          if next_v != "" {
            s.push_str(": ");
            s.push_str(&next_v);
          }
        }}};
        
        s.push_str(&cls.name.str());
        s.push_str(" { ");
        if let Some(p) = name.next() {
          next!(p);
        }
        for p in name {
          s.push_str(", ");
          next!(p);
        }
        s.push_str(" }");
        s
      }
      Ninst(inst)=> 
        format!("{} {{ Native }}", &unsafe{&*inst.cls}.name.str()),
      Sym(s)=> {
        use crate::primitive::sym::Symbol;
        let t = match s {
          Symbol::IterEnd=> "迭代结束",
          Symbol::Reserved=> "未使用"
        };
        format!("Sym {{ {} }}", t)
      }
    }
  }
}

/// 针对函数的枚举
#[derive(Debug, Clone)]
pub enum Function {
  // Native模块或Runtime提供的Rust函数
  Native(crate::native::NativeFn),
  // 绑定了self的原生函数
  NativeMethod(crate::native::BoundNativeMethod),
  // 脚本定义的本地函数
  Local(LocalFunc),
  // 使用extern语句得到的C函数
  Extern(ExternFunc)
}

/// 参数声明
#[derive(Debug, Clone)]
pub struct ArgDecl {
  pub name: Interned,
  pub t: KsType,
  pub default: Litr
}

/// 未绑定作用域的本地定义函数
#[derive(Debug, Clone)]
pub struct LocalFuncRaw {
  pub argdecl: Vec<ArgDecl>, 
  pub stmts: Statements
}

/// 本地函数指针
#[derive(Debug, Clone)]
pub struct LocalFunc {
  /// pointer
  pub ptr:*const LocalFuncRaw,
  /// 来自的作用域
  pub scope: Scope,
  /// 是否绑定了self (如果直接绑定了字面量将会持有其所有权)
  pub bound: Option<Box<CalcRef>>,
}
impl LocalFunc {
  /// 将本地函数定义和作用域绑定
  pub fn new(ptr:*const LocalFuncRaw, scope: Scope)-> Self {
    LocalFunc{
      ptr,
      scope,
      bound: None
    }
  }
}
impl std::ops::Deref for LocalFunc {
  type Target = LocalFuncRaw;
  fn deref(&self) -> &Self::Target {
    unsafe {&*self.ptr}
  }
}

/// 插件只有一个Native类型
#[derive(Debug, Clone)]
pub struct ExternFunc {
  pub argdecl: Vec<ArgDecl>, 
  pub ptr: usize,
}

/// 类实例
#[derive(Debug, Clone)]
pub struct Instance {
  pub cls: *const ClassDef,
  pub v: Box<[Litr]>
}

/// Key语言内的类型声明
/// 
/// 模块不能获取程序上下文，因此KsType对Native模块无意义
#[derive(Debug, Clone)]
pub enum KsType {
  Any,
  Int,
  Uint,
  Float,
  Bool,
  Func, 
  Str,
  Buffer,
  List,
  Obj,
  Class(Interned)
}


impl Scanner<'_> {
  /// 解析一段字面量
  /// 
  /// 同时解析一元运算符
  pub fn literal(&self)-> Expr {
    let first = self.cur();
    let len = self.src.len();
    let mut i = self.i();
  
    macro_rules! match_unary {($o:expr) => {{
      self.next();
      let right = Box::new(self.literal());
      Expr::Unary{right,op:$o}
    }}}
  
    match first {
      // 一元运算符
      b'-' => match_unary!(b'-'),
      b'!' => match_unary!(b'!'),
  
      // 解析字符字面量
      b'"' => {
        i += 1;
        while self.src[i] != b'"' {
          i += 1;
          if i >= len {self.err("未闭合的\"。")}
        }
        let s = String::from_utf8_lossy(&self.src[(self.i()+1)..i]);
        self.set_i(i+1);
        Expr::Literal(Litr::Str(s.to_string()))
      }
  
      // 解析带转义的字符串
      b'`' => {
        i += 1;
        let mut start = i; // 开始结算的起点
        let mut vec = Vec::<u8>::new();
  
        loop {
          let c = self.src[i];
          match c {
            b'`' => break,
            b'\\'=> {
              use charts::escape;
  
              // 结算一次
              vec.extend_from_slice(&self.src[start..i]);
  
              i += 1;
              // 先测试转义换行符
              macro_rules! escape_enter {() => {{
                i += 1;
                while self.src[i] == b' ' {
                  i += 1;
                }
              }}}
              let escaper = self.src[i];
              match escaper {
                b'\r'=> {
                  i += 1;
                  escape_enter!();
                }
                b'\n'=> escape_enter!(),
                // 非换行符就按转义表转义
                _=> {
                  let escaped = escape(escaper);
                  if escaped == 255 {
                    self.err(&format!("错误的转义符:{}", String::from_utf8_lossy(&[escaper])));
                  }
                  vec.push(escaped);
                  i += 1;
                }
              }
  
              // 更新结算起点
              start = i;
            }
            _=> i += 1
          }
          if i >= len {self.err("未闭合的`。")}
        }
  
        // 结算 结算起点到末尾
        vec.extend_from_slice(&self.src[start..i]);
        let str = String::from_utf8(vec)
          .expect(&format!("字符串含非法字符 解析错误({})",self.line()));
  
        self.set_i(i + 1);
        Expr::Literal(Litr::Str(str))
      }
  
      // 解析'buffer'
      b'\'' => {
        i += 1;
        let mut start = i; // 开始结算的起点
        let mut vec = Vec::<u8>::new();
  
        /// 解析{hex}
        /// 
        /// 用宏是因为嵌套太深了看着很难受
        macro_rules! parse_hex {() => {{
          // 结算左大括号之前的内容
          vec.extend_from_slice(&self.src[start..i]);
          i += 1;
          let mut braced = i; // 大括号的起点
          // 以i为界限，把hex部分切出来
          loop {
            let char = self.src[i];
            match char {
              b'0'..=b'9'|b'a'..=b'f'|b'A'..=b'F'|b'\n'|b'\r'|b' ' => i += 1,
              b'}' => break,
              _=> self.err(&format!("十六进制非法字符:{}",String::from_utf8_lossy(&[char])))
            };
            if i >= len {self.err("未闭合的}")}
          };
  
          // 结算起点延后到大括号后面
          start = i + 1;
  
          // 处理hex
          let mut hex = Vec::with_capacity(i-braced);
          while braced < i {
            // 清除空格
            while matches!(self.src[braced],b'\n'|b'\r'|b' ') {
              braced += 1;
              if braced >= i {break}
            };
            if braced >= i {
              self.err("未闭合的}")
            }
  
            let res:Result<u8,_>;
            let a = self.src[braced];
            if braced >= i {break;}
  
            braced += 1;
            if braced < i {
              let b = self.src[braced];
              braced += 1;
              res = u8::from_str_radix(&String::from_utf8_lossy(&[a,b]), 16);
            }else {
              res = u8::from_str_radix(&String::from_utf8_lossy(&[a]), 16)
            }
  
            match res {
              Ok(n)=> hex.push(n),
              Err(_)=> self.err("十六进制解析:不要把一个Byte的两个字符拆开")
            }
          }
          vec.append(&mut hex);
        }}}
  
        loop {
          let char = self.src[i];
          match char {
            b'\'' => break,
            // 十六进制解析
            b'{' => parse_hex!(),
            _=> i += 1
          }
          if i >= len {self.err("未闭合的'。")}
        }
        // 结算 结算起点到末尾
        vec.extend_from_slice(&self.src[start..i]);
  
        self.set_i(i+1);
        Expr::Literal(Litr::Buffer(vec))
      }
  
      // 解析数字字面量
      b'0'..=b'9' => {
        let mut is_float = false;
        while i < len {
          match self.src[i] {
            b'.'=> is_float = true,
            0x30..=0x39 | b'e' | b'E' => {}
            _=> break
          }
          i += 1;
        }
  
        let str = String::from_utf8(self.src[self.i()..i].to_vec()).unwrap();
        use Litr::*;
        macro_rules! parsed {
          ($t:ty, $i:ident) => {{
            let n: Result<$t,_> = str.parse();
            match n {
              Err(e)=> {
                panic!("无法解析数字:{} 解析错误({})\n  {}",str,self.line(),e)
              }
              Ok(n)=> {
                self.next();
                return Expr::Literal($i(n));
              }
            }
          }}
        }
  
        self.set_i(i);
        if i < len {
          let cur = self.src[i];
          match cur {
            b'l' => parsed!(f64, Float),
            b'u' => parsed!(usize, Uint),
            b'i'=> parsed!(isize, Int),
            _=> {}
          }
        }
        self.set_i(i-1);
  
        if is_float {
          parsed!(f64, Float)
        }else {
          parsed!(isize, Int)
        }
      },
  
      // 解析List
      b'['=> {
        self.next();
        self.spaces();
  
        let mut ls = Vec::new();
        loop {
          let e = self.expr();
          if let Expr::Empty = e {
            break;
          }
          ls.push(e);
          self.spaces();
          if self.cur() != b',' {
            break;
          }
          self.next();
        }
        if self.i() >= self.src.len() || self.cur() != b']' {
          if self.cur() == b',' {
            self.err("列表不允许空元素");
          }
          self.err("未闭合的右括号']'。");
        }
        self.next();
        Expr::List(ls)
      }

      // 解析对象
      b'{'=> Expr::Obj(self.obj()),
  
      // 解析字面量或变量
      _=> {
        let id_res = self.ident();
        if let Some(id) = id_res {
          match &*id {
            b"true"=> Expr::Literal(Litr::Bool(true)),
            b"false"=> Expr::Literal(Litr::Bool(false)),
            b"self"=> Expr::Kself,
            b"uninit"=> Expr::Literal(Litr::Uninit),
            _=> {
              self.spaces();
              if self.cur() == b'{' && id[0].is_ascii_uppercase() {
                let decl = self.obj();
                return Expr::NewInst{
                  cls: intern(id),
                  val: decl
                }
              }
              Expr::Variant(intern(id))
            }
          }
        }else {
          Expr::Empty
        }
      }
    }
  }

  /// 解析对象表达式
  fn obj(&self)-> Vec<(Interned,Expr)> {
    self.next();
    self.spaces();
    let mut decl = Vec::new();
    while let Some(id) = self.ident() {
      let v = if self.cur() == b':' {
        self.next();
        self.expr()
      }else {Expr::Literal(Litr::Uninit)};
      decl.push((intern(id),v));

      if self.cur() == b',' {
        self.next()
      }
      self.spaces();
    }

    if self.cur() != b'}' {
      self.err("未闭合的大括号")
    };
    self.next();
    decl
  }
}

