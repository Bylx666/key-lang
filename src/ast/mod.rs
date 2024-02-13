//! 抽象语法树
//! 
//! 是沟通scanner和runtime的桥梁，进行语法结构的定义，本身不做事
//! 
//! Native模块只支持了Rust，所以不需要repr(C)

mod class;
pub use class::*;
mod stmt;
pub use stmt::*;
mod expr;
pub use expr::*;

use crate::runtime::{Module, Scope};
use crate::intern::Interned;

use std::collections::HashMap;


/// 变量或字面量
#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Box<Function>), // extern和Func(){} 都属于Func直接表达式
  Str    (Box<String>),
  Buffer (Box<Vec<u8>>),
  List   (Box<Vec<Litr>>),
  Obj,
  Inst   (Box<Instance>)
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
        match **f {
          Function::Local(_)=> "<Local Function>".to_owned(),
          Function::Extern(_)=> "<Extern Function>".to_owned(),
          _=> "<Builtin Function>".to_owned()
        }
      }
      Str(s)=> (**s).clone(),
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
      Obj=> format!("obj"),
      Inst(i)=> {
        let cls = unsafe{&*i.cls};
        let mut v = i.v.iter();
        let mut str = String::new();
        str.push_str("Instance { ");
        for p in cls.props.iter() {
          str.push_str(&p.name.str());
          str.push_str(": ");
          str.push_str(&v.next().unwrap().str());
          str.push_str(", ");
        }
        str.push_str(" }");
        str
      }
    }
  }
}


/// 针对函数的枚举
#[derive(Debug, Clone)]
pub enum Function {
  // Native模块或Runtime提供的Rust函数
  Native(fn(Vec<Litr>)-> Litr),
  // 脚本定义的本地函数
  Local(Box<LocalFunc>),
  // class X {.x()}定义的方法，提供一个ClassDef指针来判断实例是否是该类型
  Method(Box<(*const ClassDef, LocalFunc)>),
  // class X {x()}定义的静态方法，提供*mut Module来判断是否能访问私有成员
  Static(Box<(*mut Module, LocalFunc)>),
  // 使用extern语句得到的C函数
  Extern(Box<ExternFunc>)
}


/// 未绑定作用域的本地定义函数
#[derive(Debug, Clone)]
pub struct LocalFuncRaw {
  pub argdecl: Vec<(Interned, KsType)>, 
  pub stmts: Statements
}


/// 本地函数指针
#[derive(Debug, Clone)]
pub struct LocalFunc {
  /// pointer
  pub ptr:*const LocalFuncRaw,
  pub scope: Scope
}
impl LocalFunc {
  /// 将本地函数定义和作用域绑定
  pub fn new(ptr:*const LocalFuncRaw, scope: Scope)-> Self {
    LocalFunc{
      ptr,
      scope
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
  pub argdecl: Vec<(Interned, KsType)>, 
  pub ptr: usize,
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

