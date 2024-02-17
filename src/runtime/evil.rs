use super::*;

/// 解析一个语句，对应Stmt
pub fn evil(this:&mut Scope, code:&Stmt) {
  match code {
    // 只有表达式的语句
    Stmt::Expression(e)=> {
      // 如果你只是在一行里空放了一个变量就不会做任何事
      if let Expr::Variant(_)=&**e {
        return;
      }
      this.calc_ref(e);
    }
    // let语句
    Stmt::Let(a)=> {
      let mut v = this.calc(&a.val);
      // 不检查变量是否存在是因为寻找变量的行为是反向的
      this.vars.push((a.id, v));
    }
    // 块语句
    Stmt::Block(s)=> {
      let mut scope = Scope::new(ScopeInner {
        parent:Some(*this),
        return_to: this.return_to,
        class_defs:Vec::new(),
        class_uses:Vec::new(),
        kself: this.kself,
        vars: Vec::with_capacity(16),
        imports: this.imports,
        exports: this.exports,
        outlives: Outlives::new()
      });
      scope.run(s);
    }

    // 类型声明
    Stmt::Class(raw)=> {
      // 为函数声明绑定作用域
      let binder = |v:&ClassFuncRaw| {
        ClassFunc { name: v.name, f: LocalFunc::new(&v.f, *this), public: v.public}
      };
      let methods:Vec<_> = raw.methods.iter().map(binder).collect();
      let statics:Vec<_> = raw.statics.iter().map(binder).collect();
      let props = raw.props.clone();
      let module = this.exports;
      let clsdef = ClassDef { name:raw.name, props, statics, methods, module};
      this.class_defs.push(clsdef);
      let using = this.class_defs.last().unwrap() as *const ClassDef;
      this.class_uses.push((raw.name, Class::Local(using)));
    }
    Stmt::NativeMod(m)=> unsafe { (*this.imports).push(Module::Native(&**m)) }

    Stmt::Using(acc)=> {
      let alia = acc.0;
      match &acc.1 {
        Expr::Variant(id)=> {
          let cls = this.find_class(*id);
          this.class_uses.push((acc.0, cls));
        }
        Expr::ModClsAcc(acc)=> {
          let cls = this.find_class_in(acc.1, acc.0);
          this.class_uses.push((alia, cls));
        }
        _=> err!("class = 语句后必须是个类声明")
      }
    }
    
    // 导入模块
    Stmt::Mod(m)=> unsafe {
      (*this.imports).push(Module::Local(&**m));
    }
    Stmt::NativeMod(m)=> unsafe {
      (*this.imports).push(Module::Native(&**m));
    }

    // 导出函数 mod.
    Stmt::ExportFn(e)=> {
      // 将函数本体生命周期拉为static
      let func_raw = Box::leak(Box::new(e.1.clone()));
      let id = e.0;
      let f = LocalFunc::new(func_raw, *this);
      // 将函数定义处的作用域生命周期永久延长
      outlive::outlive_static(f.scope);
      this.vars.push((id, Litr::Func(Box::new(Function::Local(Box::new(f.clone()))))));
      unsafe{(*this.exports).funcs.push((id,f))}
    }

    // 导出类 mod:
    Stmt::ExportCls(raw)=> {
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
      let module = this.exports;
      let clsdef = ClassDef { name:raw.name, props, statics, methods, module };
      this.class_defs.push(clsdef);
      let using = this.class_defs.last().unwrap() as *const ClassDef;
      this.class_uses.push((raw.name, Class::Local(using)));

      // 将指针推到export
      let ptr = this.class_defs.last().unwrap() as *const ClassDef;
      let module = unsafe {&mut*this.exports};
      module.classes.push((raw.name,ptr))
    }

    Stmt::Return(_)=> err!("return语句不应被直接evil"),
    _=> {}
  }
}