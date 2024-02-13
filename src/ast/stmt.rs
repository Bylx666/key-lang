//! 语句
use super::*;

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

  // 定义类
  Class    (Box<ClassDefRaw>),

  Mod       (Box<ModDef>),
  ExportFn  (Box<(Interned, LocalFuncRaw)>),
  ExportCls (Box<ClassDefRaw>),

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

/// 赋值语句
#[derive(Debug, Clone)]
pub struct AssignDef {
  pub id: Interned,
  pub val: Expr
}


#[derive(Debug, Clone)]
pub struct ModDef {
  pub name: Interned,
  pub funcs: Vec<(Interned, Function)>,
  pub classes: Vec<*const ClassDef>
}