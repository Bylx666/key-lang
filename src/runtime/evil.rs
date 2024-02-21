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
      Stmt::Block(s)=> {
        let mut scope = Scope::new(ScopeInner {
          parent:Some(*self),
          return_to: self.return_to,
          class_defs:Vec::new(),
          class_uses:Vec::new(),
          kself: self.kself,
          vars: Vec::with_capacity(16),
          imports: self.imports,
          exports: self.exports,
          outlives: Outlives::new()
        });
        scope.run(s);
      }

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
        let clsdef = ClassDef { name:raw.name, props, statics, methods, module};
        self.class_defs.push(clsdef);
        let using = self.class_defs.last().unwrap() as *const ClassDef;
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
      Stmt::Mod(m)=> unsafe {
        (*self.imports).push(Module::Local(m));
      }
      Stmt::NativeMod(m)=> unsafe {
        (*self.imports).push(Module::Native(*m));
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
        let using = self.class_defs.last().unwrap() as *const ClassDef;
        self.class_uses.push((raw.name, Class::Local(using)));

        // 将指针推到export
        let ptr = self.class_defs.last().unwrap() as *const ClassDef;
        let module = unsafe {&mut*self.exports};
        module.classes.push((raw.name,ptr))
      }

      Stmt::Return(_)=> err!("return语句不应被直接evil"),
      _=> {}
    }
  }
}