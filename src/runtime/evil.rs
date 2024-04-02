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
        self.vars.push(Variant {name:a.id, v, locked:false});
      }
      // const语句
      Stmt::Const(a)=> {
        let mut v = self.calc(&a.val);
        // 和let区别就是自动锁上
        self.vars.push(Variant {name:a.id, v, locked:true});
      }
      /// 锁定语句
      Stmt::Lock(id)=> self.lock(*id),

      // 块语句
      Stmt::Block(s)=> {
        let mut sub = self.subscope();
        sub.vars = Vec::with_capacity(s.vars);
        sub.run(s)
      },

      // 类型声明
      Stmt::Class(cls)=> {
        // SAFETY: 此行为本身会泄露2个指针的内存, 但无伤大雅
        // 导出的函数或类在引用到该类时至少不会踩到错的指针
        let clsdef = Box::into_raw(Box::new(ClassDef { p:*cls, cx:self.clone() }));
        self.class_uses.push((unsafe{(**cls).name}, Class::Local(clsdef)));
      }

      Stmt::Using(alia, e)=> {
        match e {
          Expr::Variant(id)=> {
            let cls = self.find_class(*id).unwrap_or_else(||panic!("未定义类'{}'", id.str()));
            self.class_uses.push((*alia, cls));
          }
          Expr::ModClsAcc(s, modname)=> {
            let cls = self.find_class_in(*s, *modname);
            self.class_uses.push((*alia, cls));
          }
          _=> panic!("class = 语句后必须是个类声明")
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
        let func_raw = Box::into_raw(Box::new(f.clone()));
        let f = LocalFunc::new(func_raw, *self);
        // 将函数定义处的作用域生命周期永久延长
        outlive::increase_scope_count(f.scope);
        self.vars.push(Variant {
          name:*id, v:Litr::Func(Function::Local(f.clone())), locked:false
        });
        unsafe{(*self.exports).funcs.push((*id,f))}
      }

      // 导出类 mod:
      Stmt::ExportCls(cls)=> {
        // 将class的定义复制一份, 因为其scan的结果会在模块运行完被drop
        let name = unsafe{(**cls).name};
        // 延长作用域生命周期
        outlive::increase_scope_count(*self);
        
        let clsdef = Box::into_raw(Box::new(ClassDef { p:*cls, cx:self.clone() }));
        self.class_uses.push((name, Class::Local(clsdef)));
        
        // 将指针推到export
        let module = unsafe {&mut*self.exports};
        module.classes.push((name, clsdef))
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
      Stmt::ForIter{exec, id, iterator: iter}=> {
        use primitive::iter::LitrIterator;

        let mut iter_ = self.calc_ref(iter);
        let mut iter = LitrIterator::new(&mut iter_);
        let mut breaked = false;

        match &**exec {
          Stmt::Block(exec)=> {
            for v in iter {
              let mut scope = self.subscope();
              scope.vars = Vec::with_capacity(exec.vars);
              if scope.ended || breaked {
                outlive::scope_end(scope);
                return;
              }
              if let Some(id) = id {
                scope.vars.push(Variant {name:*id, v, locked:false});
              }
              loop_run(scope, &mut breaked, exec);
              outlive::scope_end(scope);
            }
          },

          // 禁止单语句直接用循环控制语句
          Stmt::Break=> panic!("不允许`for v:iter break`的写法"),
          Stmt::Continue=> panic!("不允许`for v:iter continue`的写法`"),
          
          // 单语句运行
          _=> if let None = id {
            let mut scope = self.subscope();
            for v in iter {
              self.evil(exec);
            }
            outlive::scope_end(scope);
          }else {
            // 指定迭代过程的变量名时不可使用单语句写法
            panic!("指定了变量名的迭代 不可使用单语句")
          }
        }
      },

      Stmt::Match=>(),

      // -
      Stmt::Break=> panic!("break不在循环体内"),
      Stmt::Continue=> panic!("continue不在循环体内"),
      Stmt::Empty=> (),
    }
  }
}

/// 判断if后的条件
fn cond(v:Litr)-> bool {
  match v {
    Litr::Bool(b)=> b,
    Litr::Uninit=> false,
    _=> panic!("条件必须为Bool或uninit")
  }
}

/// 在一个作用域开始循环
fn start_loop(mut this:Scope, mut condition:impl FnMut()-> bool, exec:&Box<Stmt>) {
  if let Stmt::Block(exec) = &**exec {
    let mut scope = this.subscope();
    scope.vars = Vec::with_capacity(exec.vars);
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
    outlive::scope_end(scope);
  // 单语句将由当前作用域代为执行,不再创建新作用域
  }else {
    match &**exec {
      Stmt::Break=> panic!("不允许`for() break`的写法"),
      Stmt::Continue=> panic!("不允许`for() continue`的写法`"),
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

  for (l, sm) in &exec.v {
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