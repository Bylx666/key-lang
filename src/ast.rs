//! 抽象语法树
//! 
//! 是沟通scanner和runtime的桥梁，进行语法结构的定义，本身不做事

use std::collections::HashMap;
use crate::intern::Interned;

/// 分号分隔的，statement语句
#[derive(Debug, Clone)]
pub enum Stmt {
  Empty,                               // 空语句

  // 赋值
  Let       (Box<Assign>),               // 变量语句
  Const     (Box<Assign>),               // 常量语句

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

/// 变量提升后的Ast作用域
#[derive(Debug, Clone, Default)]
pub struct Statements {
  /// 总行数
  pub line: usize,
  /// 类型声明
  pub types: Vec<(Interned, KsType)>,
  /// (语句, 用来报错的行数)
  pub exec: Vec<(usize, Stmt)>
}

/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Literal(Litr),                // 直接值，跳脱expr递归的终点
  Empty,

  Property (Box<Prop>),         // .运算符
  Call     (Box<Call>),       // 调用函数

  Buffer   (Box<BufDecl>),      // 未处理的Buffer表达式
  Obj      (Box<ObjDecl>),      // Obj

  // 一元运算 ! -
  Unary    (Box<UnaryCalc>),
  // 二元运算
  Binary   (Box<BinCalc>),
}

/// 变量或字面量
#[derive(Debug, Clone, Copy)]
pub enum Litr {
  Uninit,
  Variant(Interned),

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (*mut Executable), // extern和Func(){} 都属于Func直接表达式
  Str    (*mut String),
  Buffer (*mut Buf),
  Array  (*mut Vec<Litr>),
  // Struct   {targ:Ident, cont:HashMap<Ident, Exprp>},    // 直接构建结构体
}
impl Litr {
  /// 由Key编译器提供的转字符
  pub fn str(self)-> String {
    use Litr::*;
    match self {
      Uninit => String::default(),
      Int(n)=> n.to_string(),
      Uint(n)=> n.to_string(),
      Float(n)=> n.to_string(),
      Bool(n)=> n.to_string(),
      Func(f)=> {
        let f = unsafe {&*f};
        match f {
          Executable::Local(_)=> "<Local Function>".to_owned(),
          Executable::Extern(_)=> "<Extern Function>".to_owned(),
          _=> "<Builtin Function>".to_owned()
        }
      }
      Str(s)=> (unsafe{&*s}).to_owned(),
      Array(a) => {
        let mut iter = unsafe{(&*a).iter()};
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
      Buffer(b)=> format!("{:?}",(unsafe{&*b})),
      Variant(s)=> s.str().to_owned()
    }
  }
}


#[derive(Debug, Clone)]
pub struct Assign {
  pub id: Interned,
  pub val: Expr
}


#[derive(Debug, Clone)]
pub enum Executable {
  Local(LocalFunc),             // 脚本内的定义
  Extern(ExternFunc),           // 脚本使用extern获取的函数
  Runtime(fn(Vec<Litr>)-> Litr) // runtime提供的函数 
}
#[derive(Debug, Clone)]
pub struct LocalFunc {
  pub args: Vec<(Interned, KsType)>, 
  pub exec: Stmt,
}
#[derive(Debug, Clone)]
pub struct ExternFunc {
  pub args: Vec<(Interned, KsType)>, 
  pub ptr: usize,
}


#[derive(Debug, Clone)]
pub enum KsType {
  Any,
  Primitive(std::mem::Discriminant<Litr>),
  Custom(Interned)
}

#[derive(Debug, Clone)]
pub struct Prop {
  pub left: Expr,
  pub right: Interned
}

#[derive(Debug, Clone)]
pub struct BinCalc {
  pub left: Expr,
  pub right: Expr,
  pub op: Vec<u8>
}

#[derive(Debug, Clone)]
pub struct UnaryCalc {
  pub right: Expr,
  pub op: u8
}

#[derive(Debug, Clone)]
pub struct Call {
  pub args: Expr,
  pub targ: Expr
}

/// Buffer declaration
#[derive(Debug, Clone)]
pub struct BufDecl {
  pub expr: Expr,
  pub ty: Vec<u8>
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
