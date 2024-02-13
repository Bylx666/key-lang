//! Class声明和实例

use crate::intern::Interned;
use crate::runtime::Module;
use crate::ast::*;

/// 未绑定作用域的类声明
#[derive(Debug, Clone)]
pub struct ClassDefRaw {
  pub name: Interned,
  pub props: Vec<ClassProp>,
  pub methods: Vec<ClassFuncRaw>,
  pub statics: Vec<ClassFuncRaw>
}

/// 绑定作用域的类声明
#[derive(Debug, Clone)]
pub struct ClassDef {
  pub props: Vec<ClassProp>,
  pub statics: Vec<ClassFunc>,
  pub methods: Vec<ClassFunc>,
  /// 用来判断是否在模块外
  pub module: *mut Module
}

/// 类中的属性声明
#[derive(Debug, Clone)]
pub struct ClassProp {
  pub name: Interned,
  pub typ: KsType,
  pub public: bool
}

/// 类中的未绑定作用域的函数声明
#[derive(Debug,Clone)]
pub struct ClassFuncRaw {
  pub name: Interned,
  pub f: LocalFuncRaw,
  pub public: bool
}

/// 类中的函数声明
#[derive(Debug,Clone)]
pub struct ClassFunc {
  pub name: Interned,
  pub f: LocalFunc,
  pub public: bool
}

/// 类实例
#[derive(Debug, Clone)]
pub struct Instance {
  pub cls: *const ClassDef,
  pub v: Box<[Litr]>
}
