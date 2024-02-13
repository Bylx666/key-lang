//! 表达式
use super::*;

/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Empty,
  // 字面量
  Literal(Litr),
  // 变量
  Variant(Interned),
  // self
  Kself,

  // 未绑定作用域的本地函数
  LocalDecl (Box<LocalFuncRaw>),

  // .运算符
  Property  (Box<PropDecl>),
  // -.运算符
  ModFuncAcc(Box<AccessDecl>),
  // -:运算符
  ModClsAcc (Box<AccessDecl>),
  // ::运算符
  ImplAccess(Box<AccessDecl>),
  // 调用函数
  Call      (Box<CallDecl>),
  // 创建实例
  NewInst   (Box<NewDecl>),

  // 列表表达式
  List      (Box<Vec<Expr>>),
  // 对象表达式
  Obj       (Box<ObjDecl>),

  // 一元运算 ! -
  Unary     (Box<UnaryDecl>),
  // 二元运算
  Binary    (Box<BinDecl>),
}

// V 注释见Expr V

#[derive(Debug, Clone)]
pub struct PropDecl {
  pub left: Expr,
  pub right: Interned
}

#[derive(Debug, Clone)]
pub struct BinDecl {
  pub left: Expr,
  pub right: Expr,
  pub op: Box<[u8]>
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
  pub args: Vec<Expr>,
  pub targ: Expr
}

#[derive(Debug, Clone)]
pub struct NewDecl {
  pub cls: Interned,
  pub val: ObjDecl
}

#[derive(Debug, Clone)]
pub struct ObjDecl (
  pub Vec<(Interned,Expr)>
);