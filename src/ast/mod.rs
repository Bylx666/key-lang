//! 抽象语法树
//! 
//! 是沟通scanner和runtime的桥梁，进行语法结构的定义，本身不做事

use std::collections::HashMap;

pub type Ident = Vec<u8>;

/// 分号分隔的，statement语句
#[derive(Debug, Clone)]
pub enum Stmt {
  // 赋值
  Let       (Box<KsAssign>),               // 变量语句
  Const     (Box<KsAssign>),               // 常量语句
  Assign    (Box<KsAssign>),               // 赋值语句

  // Key
  // Alia      (Ident,KsAssign),               // 类型别名声明
  // Key       (HashMap<Ident, KsType>),                // 类型声明语句
  // Impl      (HashMap<Ident, KsLocalFunc>), // 方法定义语句
  Match,     // 模式匹配

  // 流程控制
  Break     (Box<Expr>),                  // 中断循环并提供返回值
  Continue,                           // 立刻进入下一次循环
  Return    (Box<Expr>),                  // 函数返回

  // 表达式作为语句
  Expression(Box<Expr>),
  Empty                               // 空语句
}


#[derive(Debug, Clone)]
pub struct KsAssign {
  pub id: Ident,
  pub val: Expr
}


/// 变量提升后的Ast作用域
#[derive(Debug, Clone, Default)]
pub struct Statements {
  /// 总行数
  pub line: usize,
  /// 类型声明
  pub types: HashMap<Ident, KsType>,
  /// (语句, 用来报错的行数)
  pub exec: Vec<(usize, Stmt)>
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
  RTVoid(fn(&Litr)),             // runtime提供的函数 (多参数会传入为Array)
  RTStr(fn(&Litr)-> String),
  RTUint(fn(&Litr)-> usize)
}

#[derive(Debug, Clone)]
pub enum KsType {
  Any,
  Struction (HashMap<Ident, Ident>)
}

/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Literal(Litr),                                    // 直接值，跳脱expr递归的终点
  Empty,                                            // 空表达式，用来报错

  // 简单系列
  Property (Box<Prop>),                           // .运算符
  Call     (Box<KsCall>),                // 调用函数

  // 真假系列
  And      (Box<BinCalc>),               // and
  Or       (Box<BinCalc>),               // or

  Neg      (Box<Expr>),                                 // let i = -x;

  // 二元运算
  Binary   (Box<BinCalc>),

  // 块系列
  Block    (Box<Statements>),                                 // 一个普通块
  If       (Box<Statements>),                                 // 条件语句
  Else     (Box<Statements>),
  Loop     (Box<Statements>),                                 // 循环
}

#[derive(Debug, Clone)]
pub struct Prop {
  pub left: Expr,
  pub right: Ident
}

#[derive(Debug, Clone)]
pub struct BinCalc {
  pub left: Expr,
  pub right: Expr,
  pub sym: Vec<u8>
}

#[derive(Debug, Clone)]
pub struct KsCall {
  pub args: Expr,
  pub targ: Expr
}

// 变量或字面量
#[derive(Debug, Clone)]
pub enum Litr {
  Variant(Box<Ident>),
  Uninit,

  Int    (isize),
  Uint   (usize),
  Byte   (u8),
  Float  (f64),
  Bool   (u8),

  Func   (Box<Executable>), // extern和Func(){} 都属于Func直接表达式
  Str    (Box<Ident>),
  Array  (Box<Vec<Litr>>),
  // Struct   {targ:Ident, cont:HashMap<Ident, Exprp>},    // 直接构建结构体
}
