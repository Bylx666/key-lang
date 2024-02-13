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
mod calc;
mod externer;


/// 运行期追踪行号
/// 
/// 只有主线程会访问，不存在多线程同步问题
static mut LINE:usize = 0;
pub fn err(s:&str)-> ! {
  panic!("{} 运行时({})", s, unsafe{LINE})
}

#[derive(Debug)]
pub struct Module {
  pub imports: Vec<ModDef>,
  pub export: ModDef
}


/// 一个运行时作用域
/// 
/// run函数需要mut因为需要跟踪行数
/// 
/// return_to是用来标志一个函数是否返回过了。
/// 如果没返回，Some()里就是返回值要写入的指针
#[derive(Debug)]
pub struct ScopeInner {
  /// 父作用域
  parent: Option<Scope>,
  /// 返回值指针,None代表已返回
  return_to: *mut Option<*mut Litr>,
  /// (变量名,值)
  vars: Vec<(Interned, Litr)>,
  /// 类型声明
  class_defs: Vec<ClassDef>,
  /// 导入和导出的模块指针
  mods: *mut Module,
  /// 该作用域生命周期会被outlive的函数延长
  outlives: outlive::Outlives
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
    use Stmt::*;
    match code {
      // 只有表达式的语句
      Expression(e)=> {
        // 如果你只是在一行里空放了一个变量就不会做任何事
        if let Expr::Variant(_)=&**e {
          return;
        }
        self.calc(e);
      }
      // let语句
      Let(a)=> {
        let mut v = self.calc(&a.val);
        // 不检查变量是否存在是因为寻找变量的行为是反向的
        self.vars.push((a.id, v));
      }
      // 块语句
      Block(s)=> {
        let mut scope = Scope::new(ScopeInner {
          parent:Some(*self),
          return_to: self.return_to,
          class_defs:Vec::new(),
          vars: Vec::with_capacity(16),
          mods: self.mods,
          outlives: Outlives::new()
        });
        scope.run(s);
      }
      // 导入模块
      Mod(m)=> {
        unsafe {
          (*self.mods).imports.push((**m).clone());
        }
      }
      // 导出给自己的模块
      Export(e)=> {
        match &**e {
          ExportDef::Func((id, f)) => {
            let f = LocalFunc::new(f,*self);
            // 将函数定义处的作用域生命周期永久延长
            outlive::outlive_static(f.scope);
            let exec = Executable::Local(Box::new(f));
            self.vars.push((*id, Litr::Func(Box::new(exec.clone()))));
            unsafe{(*self.mods).export.funcs.push((*id,exec))}
          }
        }
      }
      // 类型声明
      Class(cls)=> {
        let methods:Vec<(Interned, LocalFunc)> = cls.pub_methods.iter()
          .chain(cls.priv_methods.iter())
          .map(|v|(v.0, LocalFunc::new(&v.1, *self))).collect();
        let statics:Vec<(Interned, LocalFunc)> = cls.pub_statics.iter()
          .chain(cls.priv_statics.iter())
          .map(|v|(v.0, LocalFunc::new(&v.1, *self))).collect();
        let props = cls.props.clone();
        self.class_defs.push(ClassDef {name:cls.name,props,statics,methods});
      }
      Return(_)=> err("return语句不应被直接evil"),
      _=> {}
    }
  }

  /// 调用一个函数
  pub fn call(&mut self, call: &Box<CallDecl>)-> Litr {
    // 将参数解析为参数列表
    let args = call.args.iter().map(|e|self.calc(e)).collect();

    // 如果是直接对变量调用则不需要使用calc函数
    let mut targ_mayclone = Litr::Uninit;
    let targ = match &call.targ {
      Expr::Variant(id)=> unsafe{&*(self.var(*id) as *mut Litr)}
      Expr::Literal(l)=> l,
      _=> {
        targ_mayclone = self.calc(&call.targ);
        &targ_mayclone
      }
    };
    if let Litr::Func(exec) = targ {
      use Executable::*;
      return match &**exec {
        Native(f)=> f(args),
        Local(f)=> self.call_local(&f, args),
        Extern(f)=> self.call_extern(&f, args)
      }
    }
    err(&format!("'{:?}' 不是一个函数", targ))
  }

  /// 调用本地定义的函数
  pub fn call_local(&self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    // 将传入参数按定义参数数量放入作用域
    let mut vars = Vec::with_capacity(16);
    let mut args = args.into_iter();
    for  (name,ty) in f.argdecl.iter() {
      let arg = args.next().unwrap_or(Litr::Uninit);
      vars.push((*name,arg))
    }

    let mut ret = Litr::Uninit;
    let mut return_to = Some(&mut ret as *mut Litr);
    let mut scope = Scope::new(ScopeInner {
      parent:Some(f.scope),
      return_to:&mut return_to,
      class_defs:Vec::new(),
      vars,
      mods: self.mods,
      outlives: Outlives::new()
    });
    scope.run(&f.stmts);
    ret
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
pub fn top_scope(return_to:*mut Option<*mut Litr>, mods:*mut Module)-> Scope {
  let mut vars = Vec::<(Interned, Litr)>::with_capacity(16);
  vars.push((intern(b"log"), 
    Litr::Func(Box::new(Executable::Native(io::log))))
  );
  Scope::new(ScopeInner {
    parent: None, 
    return_to, 
    class_defs:Vec::new(), 
    vars, mods, 
    outlives: Outlives::new()
  })
}

