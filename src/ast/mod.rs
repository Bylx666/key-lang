use std::collections::HashMap;

pub type Statements = Vec<Statmnt>;
pub mod keywords;

pub type Ident = Vec<u8>;

/// 分号分隔的，statement语句
#[derive(Debug, Clone)]
pub enum Statmnt {
  // 赋值
  Let       (Ident,KsAssign),               // 变量语句
  Const     (Ident,KsAssign),               // 常量语句
  Assign    (Ident,KsAssign),               // 赋值语句

  // Key
  Alia      (Ident,KsAssign),               // 类型别名声明
  Key       (HashMap<Ident, Ident>),                // 类型声明语句
  Impl      (HashMap<Ident, KsLocalFunc>), // 方法定义语句

  // 流程控制
  Break     (Exprp),                  // 中断循环并提供返回值
  Continue,                           // 立刻进入下一次循环
  Return    (Exprp),                  // 函数返回

  // 表达式作为语句
  Expression(Exprp),
  Empty                               // 空语句
}

#[derive(Debug, Clone)]
pub struct KsAssign {
  pub val: Exprp, 
  pub typ: Ident
}

#[derive(Debug, Clone)]
pub struct KsLocalFunc {
  pub args: HashMap<Ident, Ident>,  // 左名右类
  pub ret: Ident,  // 返回值
  pub exec: Statements,
}
#[derive(Debug, Clone)]
pub enum Executable {
  Local(KsLocalFunc),             // 脚本内的定义
  Extern(Ident),                 // 脚本使用extern获取的函数
  RTVoid(fn(&Vec<Imme>)),        // runtime提供的函数
  RTStr(fn(&Vec<Imme>)-> String),
  RTUint(fn(&Vec<Imme>)-> usize)
}

#[derive(Debug, Clone)]
pub enum KsType {
  Any,
  Struction (Ident)
}

pub type Exprp = Box<Expr>;

/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
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

  // 块系列
  Block    (Statements),                                 // 一个普通块
  If       (Statements),                                 // 条件语句
  Else     (Statements),
  Loop     (Statements),                                 // 循环
  // Match,     // 模式匹配

}

// 直接数值，直接指向变量或字面量
#[derive(Debug, Clone)]
pub enum Imme {
  Variant(Ident),
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Executable),       // extern和Func(){} 都属于Func直接表达式
  Str    (Ident),
  Array  (Vec<Ident>),
  Struct   {targ:Ident, cont:HashMap<Ident, Exprp>},    // 直接构建结构体
}
