//! 抽象语法树
//! 
//! 是沟通scanner和runtime的桥梁，进行语法结构的定义，本身不做事
//! 
//! Native模块只支持了Rust，所以不需要repr(C)

pub use crate::runtime::Scope;

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

  // 定义类
  Class    (Box<ClassDefRaw>),

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

/// 赋值语句
#[derive(Debug, Clone)]
pub struct AssignDef {
  pub id: Interned,
  pub val: Expr
}

/// 未绑定作用域的类声明
#[derive(Debug, Clone)]
pub struct ClassDefRaw {
  pub name: Interned,
  pub props: Vec<ClassProp>,
  pub pub_methods: Vec<(Interned, LocalFuncRaw)>,
  pub priv_methods: Vec<(Interned, LocalFuncRaw)>,
  pub pub_statics: Vec<(Interned, LocalFuncRaw)>,
  pub priv_statics: Vec<(Interned, LocalFuncRaw)>
}

/// 绑定作用域的类声明
#[derive(Debug, Clone)]
pub struct ClassDef {
  pub name: Interned,
  pub props: Vec<ClassProp>,
  pub statics: Vec<(Interned, LocalFunc)>,
  pub methods: Vec<(Interned, LocalFunc)>
}

/// 类中的属性声明
#[derive(Debug, Clone)]
pub struct ClassProp {
  pub name: Interned,
  pub typ: KsType,
  pub public: bool
}

/// 类实例
#[derive(Debug, Clone)]
pub struct Instance {
  pub cls: *const ClassDef,
  pub v: Box<[Litr]>
}


#[derive(Debug, Clone)]
pub struct ModDef {
  pub name: Interned,
  pub funcs: Vec<(Interned, Executable)>,
  pub classes: Vec<*const ClassDef>
}

#[derive(Debug, Clone)]
pub enum ExportDef {
  Func((Interned, LocalFuncRaw))
}



/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Empty,
  Literal(Litr),
  Variant(Interned),
  Kself,

  // 未绑定作用域的本地函数
  LocalDecl (Box<LocalFuncRaw>),

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
  // 创建实例
  NewInst   (Box<NewDecl>),

  // 列表表达式
  List      (Box<Vec<Expr>>),
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


/// 变量或字面量
#[derive(Debug, Clone)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Box<Executable>), // extern和Func(){} 都属于Func直接表达式
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
      Buffer(b)=> format!("{:?}",b),
      Obj=> format!("obj"),
      Inst(i)=> {
        let cls = unsafe{&*i.cls};
        let mut v = i.v.iter();
        let mut str = String::new();
        str.push_str(&cls.name.str());
        str.push_str(" { ");
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
pub enum Executable {
  Native(fn(Vec<Litr>)-> Litr), // runtime提供的函数 
  Local(Box<LocalFunc>),     // 脚本内的定义
  Extern(Box<ExternFunc>)   // 脚本使用extern获取的函数
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

// 本地方法指针
// #[derive(Debug, Clone)]
// pub struct LocalMethod {
//   /// pointer
//   pub ptr:*const LocalFuncRaw,
//   pub scope: Scope,
//   /// key self
//   pub kself: *mut Litr
// }
// impl LocalMethod {
//   /// 将本地函数定义和作用域绑定
//   pub fn new(ptr:*const LocalFuncRaw, scope: Scope, kself: *mut Litr)-> Self {
//     LocalMethod {
//       ptr,
//       scope,
//       kself
//     }
//   }
// }
// impl std::ops::Deref for LocalMethod {
//   type Target = LocalFuncRaw;
//   fn deref(&self) -> &Self::Target {
//     unsafe {&*self.ptr}
//   }
// }

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


#[derive(Debug, Clone)]
pub struct ObjDecl (
  pub Vec<(Interned,Expr)>
);

