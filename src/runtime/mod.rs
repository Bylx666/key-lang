//! 运行时环境
//! 
//! 将解析的ast放在实际作用域中运行

use crate::intern::{intern, Interned};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize,self};
use std::ptr::NonNull;
use crate::scan::{
  literal::*,
  stmt::*,
  expr::*
};
use crate::native::{NativeClassDef, NativeMod};

mod outlive;
pub use outlive::Outlives;

mod evil;
pub mod calc;
mod call;
mod externer;


/// 运行期追踪行号
/// 
/// 只有主线程会访问，不存在多线程同步问题
pub static mut LINE:usize = 0;
#[macro_use] macro_rules! err {($($a:expr$(,)?)*) => {
  panic!("{} 运行时({})", format_args!($($a,)*), unsafe{crate::runtime::LINE})
}}
pub(super) use err;


#[derive(Debug, Clone)]
pub enum Module {
  Native(*const NativeMod),
  Local(*const LocalMod)
}

/// 类声明，分为本地和原生类声明
#[derive(Debug, Clone)]
pub enum Class {
  Native(*const NativeClassDef),
  Local(*const ClassDef)
}


/// 一个运行时作用域
#[derive(Debug)]
pub struct ScopeInner {
  /// 父作用域
  pub parent: Option<Scope>,
  /// 返回值指针,None代表已返回
  pub return_to: *mut Option<*mut Litr>,
  /// (变量名,值)
  pub vars: Vec<(Interned, Litr)>,
  /// 类型声明(和作用域生命周期一致)
  pub class_defs: Vec<ClassDef>,
  /// 类型使用
  pub class_uses: Vec<(Interned, Class)>,
  /// self指针
  pub kself: *mut Litr,
  /// 当前脚本导入的模块
  pub imports: *mut Vec<Module>,
  /// ks本身作为模块导出的指针
  pub exports: *mut LocalMod,
  /// 该作用域生命周期会被outlive的函数延长
  pub outlives: outlive::Outlives
}


/// 作用域指针
/// 
/// 之所以把方法定义到指针上是因为垃圾回收需要确认自己的指针
/// 
/// 在结构体里写自己的指针应该是未定义行为
#[derive(Debug, Clone, Copy)]
pub struct Scope {
  pub ptr:*mut ScopeInner
}
impl Scope {
  pub fn new(s:ScopeInner)-> Self {
    Scope {
      ptr: Box::into_raw(Box::new(s))
    }
  }
  /// 确认此作用域是否为一个作用域的子作用域
  pub fn subscope_of(&self,upper:Scope)-> bool {
    let mut scope = *self;
    let upper = upper.ptr;
    if scope.ptr == upper {
      return true;
    }
    while let Some(parent) = scope.parent {
      if parent.ptr == upper {
        return true;
      }
      scope = parent;
    }
    false
  }
}
impl std::ops::Deref for Scope {
  type Target = ScopeInner;
  fn deref(&self) -> &Self::Target {
    unsafe {&*self.ptr}
  }
}
impl std::ops::DerefMut for Scope {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {&mut *self.ptr}
  }
}

impl Scope {
  /// 在此作用域运行ast代码
  /// 
  /// 此行为会根据引用计数回收作用域，在run之后再次使用Scope是未定义行为
  pub fn run(mut self, codes:&Statements) {
    for (l, sm) in &codes.0 {
      unsafe{LINE = *l;}

      // 如果子作用域返回过了，这里就会是None状态
      let return_to = unsafe{&*self.return_to};
      if let None = return_to {
        return;
      }

      // 遇到return语句就停止当前遍历
      if let Stmt::Return(expr) = sm {
        unsafe {
          if let Some(p) = return_to {
            **p = self.calc(expr);
          }
          *self.return_to = None;
        }
        outlive::scope_end(self);
        return;
      }

      self.evil(sm);
    }
    outlive::scope_end(self);
  }

  /// 在作用域找一个变量
  pub fn var(&mut self, s:Interned)-> &mut Litr {
    let inner = &mut (**self);
    for (p, v) in inner.vars.iter_mut().rev() {
      if *p == s {
        return v;
      }
    }

    if let Some(parent) = &mut inner.parent {
      return parent.var(s);
    }
    err!("无法找到变量 '{}'", s.str());
  }

  /// 在作用域找一个变量并返回其所在作用域
  pub fn var_with_scope(&mut self, s:Interned)-> (&mut Litr, Scope) {
    let scope = self.clone();
    let inner = &mut (**self);
    for (p, v) in inner.vars.iter_mut().rev() {
      if *p == s {
        return (v,scope);
      }
    }

    if let Some(parent) = &mut inner.parent {
      return parent.var_with_scope(s);
    }
    err!("无法找到变量 '{}'", s.str());
  }


  /// 在当前use过的类声明中找对应的类
  pub fn find_class(&self, s:Interned)-> Class {
    for (name, cls) in self.class_uses.iter().rev() {
      if *name == s {
        return cls.clone();
      }
    }
    if let Some(parent) = &self.parent {
      return parent.find_class(s);
    }
    err!("未定义类 '{}'", s.str());
  }
  /// 在一个模块中找一个类声明
  pub fn find_class_in(&self, modname:Interned, s: Interned)-> Class {
    let module = self.find_mod(modname);
    match module {
      Module::Local(p)=> {
        let m = unsafe {&*p};
        for (name, cls) in m.classes.iter() {
          if *name == s {
            return Class::Local(*cls);
          }
        }
        err!("模块'{}'中没有'{}'类型",modname.str(), s.str())
      }
      Module::Native(p)=> {
        let m = unsafe {&*p};
        for cls in m.classes.iter() {
          let name = unsafe {&**cls}.name;
          if name == s {
            return Class::Native(*cls);
          }
        }
        err!("原生模块'{}'中没有'{}'类型",modname.str(), s.str())
      }
    }
  }


  /// 寻找一个导入的模块
  pub fn find_mod(&self, s:Interned)-> Module {
    let imports = unsafe {&*self.imports};
    for module in imports.iter() {
      match module {
        Module::Local(p)=> {
          let m = unsafe {&**p};
          if m.name == s {
            return module.clone();
          }
        }
        Module::Native(p)=> {
          let m = unsafe {&**p};
          if m.name == s {
            return module.clone();
          }
        }
      }
    }
    err!("当前模块中没有导入'{}'模块", s.str())
  }
}


#[derive(Debug)]
pub struct RunResult {
  pub returned: Litr,
  pub exports: LocalMod,
  pub kself: Litr
}

/// 创建顶级作用域并运行一段程序
pub fn run(s:&Statements)-> RunResult {
  let mut top_ret = Litr::Uint(0);
  let mut return_to = &mut Some(&mut top_ret as *mut Litr);
  let mut imports = Vec::new();
  let mut exports = LocalMod { name: intern(b"mod"), funcs: Vec::new(), classes: Vec::new() };
  let mut kself = Litr::Uninit;
  top_scope(return_to, &mut imports, &mut exports,&mut kself).run(s);
  RunResult { returned: top_ret, exports, kself }
}

/// 创建顶级作用域
/// 
/// 自定义此函数可添加初始函数和变量
pub fn top_scope(return_to:*mut Option<*mut Litr>, imports:*mut Vec<Module>, exports:*mut LocalMod, kself:*mut Litr)-> Scope {
  let mut vars = Vec::<(Interned, Litr)>::with_capacity(16);
  vars.push((intern(b"log"), 
    Litr::Func(Function::Native(crate::primitive::std::log)))
  );
  let mut class_uses = crate::primitive::classes();

  Scope::new(ScopeInner {
    parent: None, 
    return_to, 
    class_defs:Vec::new(), 
    class_uses,
    kself,
    imports,
    exports,
    vars, 
    outlives: Outlives::new()
  })
}
