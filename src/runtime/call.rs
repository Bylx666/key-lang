use super::*;

impl Scope {
  /// 解析Expr的调用
  pub fn call(&mut self, args:&Vec<Expr>, targ:&Box<Expr>)-> Litr {
    let targ = self.calc_ref(targ);
    if let Litr::Func(exec) = &*targ {
      use Function::*;
      match exec {
        Native(f)=> {
          let args = args.iter().map(|e|self.calc_ref(e)).collect();
          f(args)
        },
        NativeMethod(f)=> {
          let args = args.iter().map(|e|self.calc_ref(e)).collect();
          (f.f)(f.bind, args)
        },
        Local(f)=> {
          let args = args.iter().map(|e|self.calc(e)).collect();
          self.call_local(&f, args)
        },
        Extern(f)=> {
          let args = args.iter().map(|e|self.calc(e)).collect();
          super::externer::call_extern(&f, args)
        }
      }
    }else {
      err!("'{}' 不是一个函数", targ.str())
    }
  }

  /// 实际调用一个local function
  pub fn call_local(&self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    // 将传入参数按定义参数数量放入作用域
    let mut vars = Vec::with_capacity(16);
    let mut args = args.into_iter();
    for argdecl in f.argdecl.iter() {
      let arg = args.next().unwrap_or(argdecl.default.clone());
      vars.push((argdecl.name, arg));
    }
    // 如果函数被bind了就用bound值，否则继续沿用上级self
    let kself = if let Some(s) = f.bound {s}else {self.kself};

    let mut ret = Litr::Uninit;
    let mut scope = Scope::new(ScopeInner {
      parent:Some(f.scope),
      return_to:&mut ret,
      class_defs:Vec::new(),
      class_uses:Vec::new(),
      kself,
      vars,
      imports: self.imports,
      exports: self.exports,
      outlives: Outlives::new(),
      ended: false
    });
    scope.run(&f.stmts);
    ret
  }
}

