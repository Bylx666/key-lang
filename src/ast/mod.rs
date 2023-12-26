use std::collections::HashMap;

pub type Statements = Vec<Statmnt>;
pub mod keywords;

use crate::Ident;

/// 分号分隔的，statement语句
#[derive(Debug)]
pub enum Statmnt {
  // 赋值
  Let       (KsAssign),               // 变量语句
  Const     (KsAssign),               // 常量语句
  Assign    (KsAssign),               // 赋值语句

  // Key
  Alia      (KsAssign),               // 类型别名声明
  Key       (HashMap<Ident, Ident>),                // 类型声明语句
  Impl      (HashMap<Ident, KsFunc>), // 方法定义语句

  // 流程控制
  Break     (Exprp),                  // 中断循环并提供返回值
  Continue,                           // 立刻进入下一次循环
  Return    (Exprp),                  // 函数返回

  // 表达式作为语句
  Expression(Exprp),
  Empty                               // 空语句
}

#[derive(Debug)]
pub struct KsAssign {
  pub name: Ident, 
  pub val: Exprp, 
  pub typ: Ident
}

#[derive(Debug)]
pub struct KsFunc {
  pub args: HashMap<Ident, Ident>,  // 左名右类
  pub ret: Ident,  // 返回值
  pub exec: Statements,
}

type Exprp = Box<Expr>;

/// 可以出现在任何右值的，expression表达式
#[derive(Debug)]
pub enum Expr {
  Immediate(Imme),                                  // 直接值，跳脱expr递归的终点

  // 简单系列
  Path,                                             // ::运算符
  Call     {args:Vec<Exprp>, targ:Ident},           // 调用函数
  CalcA    {left:Exprp, right:Exprp, op:u8},        // */%
  CalcB    {left:Exprp, right:Exprp, op:u8},        // +-
  And      {left:Exprp, right:Exprp},               // and
  Or       {left:Exprp, right:Exprp},               // or
  Neg      (Exprp),                                 // let i = -x;
  Struct   {targ:Ident, cont:HashMap<Ident, Exprp>},// 直接构建结构体

  // 块系列
  Block    (Statements),                                 // 一个普通块
  If       (Statements),                                 // 条件语句
  Else     (Statements),
  Loop     (Statements),                                 // 循环
  // Match,     // 模式匹配

}

// 直接数值，直接指向变量或字面量
#[derive(Debug)]
pub enum Imme {
  Variant(Ident),
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  // Func   (KsFunc),
  Str    (Ident),
  Array  (Vec<Ident>),
}
