use crate::primitive;

use super::*;

impl Scope {
  /// 解析一个语句，对应Stmt
  pub fn evil(&mut self, code:&Stmt) {
    match code {
      // 只有表达式的语句
      Stmt::Expression(e)=> {
        // 如果你只是在一行里空放了一个变量就不会做任何事
        if let Expr::Variant(_) = e {
          return;
        }
        self.calc_ref(e);
      }
      // let语句
      Stmt::Let(a)=> {
        let mut v = self.calc(&a.val);
        // 不检查变量是否存在是因为寻找变量的行为是反向的
        self.vars.push((a.id, v));
      }
      // 块语句
      Stmt::Block(s)=> self.subscope().run(s),

      // 类型声明
      Stmt::Class(raw)=> {
        // 为函数声明绑定作用域
        let binder = |v:&ClassFuncRaw| {
          ClassFunc { name: v.name, f: LocalFunc::new(&v.f, *self), public: v.public}
        };
        let methods:Vec<_> = raw.methods.iter().map(binder).collect();
        let statics:Vec<_> = raw.statics.iter().map(binder).collect();
        let props = raw.props.clone();
        let module = self.exports;
        let clsdef = ClassDef { name:raw.name, props, statics, methods, module };
        self.class_defs.push(clsdef);
        let using = self.class_defs.last_mut().unwrap() as *mut ClassDef;
        self.class_uses.push((raw.name, Class::Local(using)));
      }

      Stmt::Using(alia, e)=> {
        match e {
          Expr::Variant(id)=> {
            let cls = self.find_class(*id);
            self.class_uses.push((*alia, cls));
          }
          Expr::ModClsAcc(s, modname)=> {
            let cls = self.find_class_in(*s, *modname);
            self.class_uses.push((*alia, cls));
          }
          _=> err!("class = 语句后必须是个类声明")
        }
      }
      
      // 导入模块
      Stmt::Mod(name, m)=> unsafe {
        (*self.imports).push((*name, Module::Local(*m)));
      }
      Stmt::NativeMod(name, m)=> unsafe {
        (*self.imports).push((*name, Module::Native(*m)));
      }

      // 导出函数 mod.
      Stmt::ExportFn(id, f)=> {
        // 将函数本体生命周期拉为static
        let func_raw = Box::leak(Box::new(f.clone()));
        let f = LocalFunc::new(func_raw, *self);
        // 将函数定义处的作用域生命周期永久延长
        outlive::outlive_static(f.scope);
        self.vars.push((*id, Litr::Func(Function::Local(f.clone()))));
        unsafe{(*self.exports).funcs.push((*id,f))}
      }

      // 导出类 mod:
      Stmt::ExportCls(raw)=> {
        // 为函数声明绑定作用域
        let binder = |v:&ClassFuncRaw| {
          // 延长函数体生命周期
          let ptr = Box::leak(Box::new(v.f.clone()));
          ClassFunc { name: v.name, f: LocalFunc::new(ptr, *self), public: v.public}
        };
        // 延长作用域生命周期
        outlive::outlive_static(*self);

        let methods:Vec<_> = raw.methods.iter().map(binder).collect();
        let statics:Vec<_> = raw.statics.iter().map(binder).collect();
        let props = raw.props.clone();
        let module = self.exports;
        let clsdef = ClassDef { name:raw.name, props, statics, methods, module };
        self.class_defs.push(clsdef);
        let using = self.class_defs.last_mut().unwrap() as *mut ClassDef;
        self.class_uses.push((raw.name, Class::Local(using)));

        // 将指针推到export
        let ptr = self.class_defs.last_mut().unwrap() as *mut ClassDef;
        let module = unsafe {&mut*self.exports};
        module.classes.push((raw.name,ptr))
      }

      // 返回一个值
      Stmt::Return(expr)=> {
        // 遇到return语句就停止当前遍历
        // 并将返回值指针相同(在同一函数内的作用域)设为已结束
        unsafe{*self.return_to = self.calc(expr)};
        self.ended = true;
        let mut scope = *self;
        while let Some(mut s) = scope.parent {
          if s.return_to != self.return_to {break;}
          s.ended = true;
          scope = s;
        }
      },

      // if else
      Stmt::If { condition, exec, els }=> {
        if cond(self.calc(condition)) {
          self.evil(exec)
        }else if let Some(els) = els {
          self.evil(els)
        }
      }

      // for ()语句
      Stmt::ForWhile { condition, exec }=>
        start_loop(*self, ||cond(self.calc(condition)), exec),

      // for!语句
      Stmt::ForLoop(exec)=> start_loop(*self, ||true, exec),

      // for v:iter语句
      Stmt::ForIter{exec, id, iterator}=> {
        use primitive::iter::LitrIterator;
        let mut calced = self.calc_ref(iterator);
        let mut scope = self.subscope();
        let mut breaked = false;
        match &**exec {
          Stmt::Block(exec)=> {
            for v in LitrIterator::new(&mut calced) {
              if scope.ended || breaked {
                outlive::scope_end(scope);
                return;
              }
              scope.vars.clear();
              if let Some(id) = id {
                scope.vars.push((*id, v));
              }
              scope.class_uses.clear();
              loop_run(scope, &mut breaked, exec)
            }
          },
          _=> for v in LitrIterator::new(&mut calced) {
            if scope.ended {
              return;
            }
            scope.vars.clear();
            if let Some(id) = id {
              scope.vars.push((*id, v));
            }
            scope.evil(exec);
          }
          Stmt::Break=> err!("不允许`for v:iter break`的写法"),
          Stmt::Continue=> err!("不允许`for v:iter continue`的写法`"),
        }
        scope.ended = true;
        outlive::scope_end(scope);
      },

      Stmt::Match=>(),

      // -
      Stmt::Break=> err!("break不在循环体内"),
      Stmt::Continue=> err!("continue不在循环体内"),
      Stmt::Empty=> (),
    }
  }
}

/// 判断if后的条件
fn cond(v:Litr)-> bool {
  match v {
    Litr::Bool(b)=> b,
    Litr::Uninit=> false,
    _=> err!("条件必须为Bool或uninit")
  }
}

/// 在一个作用域开始循环
fn start_loop(mut this:Scope, mut condition:impl FnMut()-> bool, exec:&Box<Stmt>) {
  // 用重置作用域代替重新创建作用域
  if let Stmt::Block(exec) = &**exec {
    let mut scope = this.subscope();
    let mut breaked = false;
    while condition() {
      if scope.ended || breaked {
        outlive::scope_end(scope);
        return;
      }
      // 重置此作用域
      scope.vars.clear();
      scope.class_uses.clear();
      loop_run(scope, &mut breaked, exec);
    }
    scope.ended = true;
    outlive::scope_end(scope);
  // 单语句将由当前作用域代为执行,不再创建新作用域
  }else {
    match &**exec {
      Stmt::Break=> err!("不允许`for() break`的写法"),
      Stmt::Continue=> err!("不允许`for() continue`的写法`"),
      _=> while condition() {
        if this.ended {
          return;
        }
        this.evil(exec);
      }
    }
  }
}

/// 以循环模式运行一段语句
fn loop_run(mut scope:Scope,breaked:&mut bool,exec:&Statements) {
  // 对于单Stmt的run实现
  macro_rules! loop_run_stmt {($stmt:expr)=>{{
    match $stmt {
      Stmt::Block(exec)=> {
        let mut s = scope.subscope();
        loop_run(s, breaked, exec);
        s.ended = true;
        outlive::scope_end(s);
      },
      Stmt::Break=> return *breaked = true,
      Stmt::Continue=> return,
      _=> scope.evil($stmt)
    };
  }}}

  for (l, sm) in &exec.0 {
    // 如果中途遇到return或者break就停止
    if scope.ended || *breaked {
      return;
    }
    match sm {
      Stmt::Break=> return *breaked = true,
      Stmt::Continue=> return,
      // 把直属该for下的块拦截,检测break和continue
      Stmt::Block(v)=> loop_run(scope, breaked, exec),
      Stmt::If { condition, exec, els }=> {
        if cond(scope.calc(condition)) {
          loop_run_stmt!(&**exec)
        }else if let Some(els) = els {
          loop_run_stmt!(&**els)
        }
      },
      _=> {
        unsafe{LINE = *l;}
        scope.evil(sm);
      }
    }
  }
}