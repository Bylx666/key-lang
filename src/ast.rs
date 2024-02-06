//! 抽象语法树
//! 
//! 是沟通scanner和runtime的桥梁，进行语法结构的定义，本身不做事
//! 
//! Native模块只支持了Rust，所以不需要repr(C)

pub use crate::runtime::{LocalFunc,Scope,ScopeInner};

use std::collections::HashMap;
use crate::intern::Interned;

/// 语句列表
#[derive(Debug, Clone, Default)]
pub struct Statements (
  pub Vec<(usize, Stmt)>
);


/// 分号分隔的，statement语句
#[derive(Debug, Clone)]
pub enum Stmt {
  Empty,

  // 赋值
  Let       (Box<AssignDef>),

  // 定义结构
  Struct    (Box<StructDef>),

  Mod       (Box<ModDef>),
  Export    (Box<ExportDef>),

  // Key
  // Key       (HashMap<Ident, KsType>),                // 类型声明语句
  // Impl      (HashMap<Ident, KsLocalFunc>), // 方法定义语句
  Match,     // 模式匹配

  // 块系列
  Block    (Box<Statements>),   // 一个普通块
  If       (Box<Statements>),   // 条件语句
  Loop     (Box<Statements>),   // 循环

  // 流程控制
  Break     (Box<Expr>),                  // 中断循环并提供返回值
  Continue,                           // 立刻进入下一次循环
  Return    (Box<Expr>),                  // 函数返回

  // 表达式作为语句
  Expression(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct AssignDef {
  pub id: Interned,
  pub val: Expr
}

#[derive(Debug, Clone)]
pub struct StructDef (
  pub Vec<(Interned,KsType)>
);

#[derive(Debug, Clone)]
pub struct ModDef {
  pub name: Interned,
  pub funcs: Vec<(Interned, Executable)>
}

#[derive(Debug, Clone)]
pub enum ExportDef {
  Func((Interned, LocalFuncInner))
}


/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Empty,
  Literal(Litr),
  Variant(Interned),

  // 未绑定作用域的本地函数
  LocalDecl (Box<LocalFuncInner>),

  // .运算符
  Property  (Box<PropDecl>),
  // -.运算符
  ModFuncAcc(Box<AccessDecl>),
  // -:运算符
  ModStruAcc(Box<AccessDecl>),
  // ::运算符
  ImplAccess(Box<AccessDecl>),
  // 调用函数
  Call      (Box<CallDecl>),

  // 未处理的Buffer表达式
  Buffer    (Box<BufDecl>),
  Obj       (Box<ObjDecl>),

  // 一元运算 ! -
  Unary     (Box<UnaryDecl>),
  // 二元运算
  Binary    (Box<BinDecl>),
}

#[derive(Debug, Clone)]
pub struct PropDecl {
  pub left: Expr,
  pub right: Interned
}

#[derive(Debug, Clone)]
pub struct BinDecl {
  pub left: Expr,
  pub right: Expr,
  pub op: Vec<u8>
}

#[derive(Debug, Clone)]
pub struct UnaryDecl {
  pub right: Expr,
  pub op: u8
}

#[derive(Debug, Clone)]
pub struct AccessDecl {
  pub left: Interned,
  pub right: Interned
}

#[derive(Debug, Clone)]
pub struct CallDecl {
  pub args: Expr,
  pub targ: Expr
}

/// Buffer declaration
#[derive(Debug, Clone)]
pub struct BufDecl {
  pub expr: Expr,
  pub ty: Vec<u8>
}


/// 变量或字面量
#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,
  Ref    (*mut Litr),

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Box<Executable>), // extern和Func(){} 都属于Func直接表达式
  Str    (Box<String>),
  Buffer (Box<Buf>),
  List  (Box<Vec<Litr>>),
  // Struct   {targ:Ident, cont:HashMap<Ident, Exprp>},    // 直接构建结构体
}
impl Litr {
  /// 由Key编译器提供的转字符
  pub fn str(&self)-> String {
    use Litr::*;
    match self {
      Uninit => String::default(),
      Ref(_)=> "<Reference>".to_string(),
      Int(n)=> n.to_string(),
      Uint(n)=> n.to_string(),
      Float(n)=> n.to_string(),
      Bool(n)=> n.to_string(),
      Func(f)=> {
        match **f {
          Executable::Local(_)=> "<Local Function>".to_owned(),
          Executable::Extern(_)=> "<Extern Function>".to_owned(),
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
      Buffer(b)=> format!("{:?}",b)
    }
  }
}


/// 针对函数的枚举
#[derive(Debug, Clone)]
pub enum Executable {
  Local(LocalFunc),     // 脚本内的定义
  Extern(Box<ExternFunc>),           // 脚本使用extern获取的函数
  Native(fn(Vec<Litr>)-> Litr) // runtime提供的函数 
}


/// 本地定义函数
#[derive(Debug, Clone)]
pub struct LocalFuncInner {
  pub argdecl: Vec<(Interned, KsType)>, 
  pub exec: Statements,
  pub scope: Scope
}

/// 插件只有一个Native类型
#[derive(Debug, Clone)]
pub struct ExternFunc {
  pub argdecl: Vec<(Interned, KsType)>, 
  pub ptr: usize,
}


/// Key语言内的类型声明
/// 
/// 插件不能获取程序上下文，因此KsType对插件无意义
#[derive(Debug, Clone)]
pub enum KsType {
  Any,
  Primitive(std::mem::Discriminant<Litr>),
  Custom(Interned)
}


#[derive(Debug, Clone)]
pub enum Buf {
  U8(Vec<u8>),
  U16(Vec<u16>),
  U32(Vec<u32>),
  U64(Vec<u64>),
  I8(Vec<i8>),
  I16(Vec<i16>),
  I32(Vec<i32>),
  I64(Vec<i64>),
  F32(Vec<f32>),
  F64(Vec<f64>)
}

#[derive(Debug, Clone)]
pub struct ObjDecl (
  Vec<(Interned,Expr)>
);


pub struct StructElem {
  
}

pub enum StructElemType {
  Uint8,
  Uint16,
  Uint32,
  Uint,
  Int8,
  Int16,
  Int32,
  Int,
  Float32,
  Float,
  Bool,
  Strp,
  Funcp,
  Bufferp,
  Arrayp,
  Structp
}
