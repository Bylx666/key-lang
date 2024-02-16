use super::*;

/// 解析一个语句，对应Stmt
pub fn evil(this:&mut Scope, code:&Stmt) {
  use Stmt::*;
  match code {
    // 只有表达式的语句
    Expression(e)=> {
      // 如果你只是在一行里空放了一个变量就不会做任何事
      if let Expr::Variant(_)=&**e {
        return;
      }
      this.calc_ref(e);
    }
    // let语句
    Let(a)=> {
      let mut v = this.calc(&a.val);
      // 不检查变量是否存在是因为寻找变量的行为是反向的
      this.vars.push((a.id, v));
    }
    // 块语句
    Block(s)=> {
      let mut scope = Scope::new(ScopeInner {
        parent:Some(*this),
        return_to: this.return_to,
        class_defs:Vec::new(),
        class_uses:Vec::new(),
        kself: this.kself,
        vars: Vec::with_capacity(16),
        module: this.module,
        outlives: Outlives::new()
      });
      scope.run(s);
    }

    // 类型声明
    Class(raw)=> {
      // 为函数声明绑定作用域
      let binder = |v:&ClassFuncRaw| {
        ClassFunc { name: v.name, f: LocalFunc::new(&v.f, *this), public: v.public}
      };
      let methods:Vec<_> = raw.methods.iter().map(binder).collect();
      let statics:Vec<_> = raw.statics.iter().map(binder).collect();
      let props = raw.props.clone();
      let module = this.module;
      let clsdef = ClassDef { name:raw.name, props, statics, methods, module};
      this.class_defs.push(clsdef);
      let using = this.class_defs.last().unwrap() as *const ClassDef;
      this.class_uses.push((raw.name, using));
    }

    Using(acc)=> {
      let alia = acc.0;
      match &acc.1 {
        Expr::Variant(id)=> {
          let cls = this.find_class(*id) as *const ClassDef;
          this.class_uses.push((acc.0, cls));
        }
        Expr::ModClsAcc(acc)=> {
          let modname = acc.0;
          let clsname = acc.1;
          for moddef in unsafe{&*this.module}.imports.iter() {
            if moddef.name == modname {
              for (name, cls) in &moddef.classes {
                if *name == clsname {
                  this.class_uses.push((alia, *cls));
                  return;
                }
              }
            }
          }
        }
        _=> err("class = 语句后必须是个类声明")
      }
    }
    
    // 导入模块
    Mod(m)=> {
      unsafe {
        (*this.module).imports.push((**m).clone());
      }
    }
    // 导出函数 mod.
    ExportFn(e)=> {
      // 将函数本体生命周期拉为static
      let func_raw = Box::leak(Box::new(e.1.clone()));
      let id = e.0;
      let f = LocalFunc::new(func_raw, *this);
      // 将函数定义处的作用域生命周期永久延长
      outlive::outlive_static(f.scope);
      this.vars.push((id, Litr::Func(Box::new(Function::Local(Box::new(f.clone()))))));
      unsafe{(*this.module).export.funcs.push((id,f))}
    }

    // 导出类 mod:
    ExportCls(raw)=> {
      // 为函数声明绑定作用域
      let binder = |v:&ClassFuncRaw| {
        // 延长函数体生命周期
        let ptr = Box::leak(Box::new(v.f.clone()));
        ClassFunc { name: v.name, f: LocalFunc::new(ptr, *this), public: v.public}
      };
      // 延长作用域生命周期
      outlive::outlive_static(*this);

      let methods:Vec<_> = raw.methods.iter().map(binder).collect();
      let statics:Vec<_> = raw.statics.iter().map(binder).collect();
      let props = raw.props.clone();
      let module = this.module;
      let clsdef = ClassDef { name:raw.name, props, statics, methods, module };
      this.class_defs.push(clsdef);
      let using = this.class_defs.last().unwrap() as *const ClassDef;
      this.class_uses.push((raw.name, using));

      // 将指针推到export
      let ptr = this.class_defs.last().unwrap();
      let module = this.module;
      unsafe{(*module).export.classes.push((raw.name,ptr))}
    }

    Return(_)=> err("return语句不应被直接evil"),
    _=> {}
  }
}