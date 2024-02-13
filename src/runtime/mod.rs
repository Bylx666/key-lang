//! 运行时环境
//! 
//! 将解析的ast放在实际作用域中运行

use crate::ast::*;
use crate::intern::{intern, Interned};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize,self};
use std::ptr::NonNull;

mod outlive;
pub use outlive::Outlives;
mod io;
mod evil;
mod calc;
mod call;
mod externer;


/// 运行期追踪行号
/// 
/// 只有主线程会访问，不存在多线程同步问题
static mut LINE:usize = 0;
pub fn err(s:&str)-> ! {
  panic!("{} 运行时({})", s, unsafe{LINE})
}

// 是runtime单方面使用的，不需要定义到ast
/// 当前Ks的模块
#[derive(Debug)]
pub struct Module {
  pub imports: Vec<ModDef>,
  pub export: ModDef
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
  /// 类型声明
  pub class_defs: Vec<ClassDef>,
  /// self指针
  pub kself: *mut Instance,
  /// 导入和导出的模块指针
  pub module: *mut Module,
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

  /// 在作用域解析一个语句
  pub fn evil(&mut self, code:&Stmt) {
    evil::evil(self, code)
  }

  /// 调用一个函数
  pub fn call(&mut self, call: &Box<CallDecl>)-> Litr {
    call::call(self, call)
  }

  /// 调用本地定义的函数
  pub fn call_local(&self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    call::call_local(self, f, args)
  }

  /// 调用method
  pub fn call_method(&self, f:&LocalFunc, kself:*mut Instance, args:Vec<Litr>)-> Litr {
    call::call_method(self, f, kself, args)
  }

  /// 调用extern函数
  pub fn call_extern(&self, f:&ExternFunc, args:Vec<Litr>)-> Litr {
    externer::call_extern(self,f,args)
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
    err(&format!("无法找到变量 '{}'", s.str()));
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
    err(&format!("无法找到变量 '{}'", s.str()));
  }


  /// 寻找一个类声明
  pub fn find_class(&self, s:Interned)-> &ClassDef {
    for cls in self.class_defs.iter().rev() {
      if cls.name == s {
        return cls;
      }
    }
    if let Some(parent) = &self.parent {
      return parent.find_class(s);
    }
    err(&format!("未定义类 '{}'", s.str()));
  }


  /// 在此作用域计算表达式的值
  /// 
  /// 调用此函数必定会复制原内容
  /// 
  /// 因此在calc前手动判断表达式是否为变量就能少复制一次了
  pub fn calc(&mut self, e:&Expr)-> Litr {
    calc::calc(self, e)
  }
}


#[derive(Debug)]
pub struct RunResult {
  pub returned: Litr,
  pub exported: ModDef
}

/// 创建顶级作用域并运行一段程序
pub fn run(s:&Statements)-> RunResult {
  let mut top_ret = Litr::Uint(0);
  let mut return_to = &mut Some(&mut top_ret as *mut Litr);
  let mut mods = Module { 
    imports: Vec::new(), 
    export: ModDef { name: intern(b"mod"), funcs: Vec::new(), classes: Vec::new() } 
  };
  top_scope(return_to, &mut mods).run(s);
  RunResult { returned: top_ret, exported: mods.export }
}

/// 创建顶级作用域
/// 
/// 自定义此函数可添加初始函数和变量
pub fn top_scope(return_to:*mut Option<*mut Litr>, module:*mut Module)-> Scope {
  let mut vars = Vec::<(Interned, Litr)>::with_capacity(16);
  vars.push((intern(b"log"), 
    Litr::Func(Box::new(Function::Native(io::log))))
  );

  let cls = ClassDef{methods:Vec::new(),name:intern(b""),props:Vec::new(),statics:Vec::new(),module};
  let mut kself = Instance {cls:&cls, v:[].into()};
  Scope::new(ScopeInner {
    parent: None, 
    return_to, 
    class_defs:Vec::new(), 
    kself: &mut kself,
    vars, module, 
    outlives: Outlives::new()
  })
}
