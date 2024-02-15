use super::*;

/// 解析Expr的调用
pub fn call(this:&mut Scope, call: &Box<CallDecl>)-> Litr {
  // 将参数解析为参数列表
  let args = call.args.iter().map(|e|this.calc(e)).collect();
  let targ = this.calc_ref(&call.targ);
  if let Litr::Func(exec) = &*targ {
    use Function::*;
    match &**exec {
      Native(f)=> f(args),
      Local(f)=> call_local(this, &f, args),
      Extern(f)=> this.call_extern(&f, args),
    }
  }else {
    err(&format!("'{}' 不是一个函数", targ.str()))
  }
}

/// 实际调用一个local function
pub fn call_local(this:&Scope, f:&LocalFunc, args:Vec<Litr>)-> Litr {
  // 将传入参数按定义参数数量放入作用域
  let mut vars = Vec::with_capacity(16);
  let mut args = args.into_iter();
  for (name,ty) in f.argdecl.iter() {
    let arg = args.next().unwrap_or(Litr::Uninit);
    vars.push((*name,arg))
  }
  // 如果函数被bind了就用bound值，否则继续沿用上级self
  let kself = if let Some(s) = f.bound {s}else {this.kself};

  let mut ret = Litr::Uninit;
  let mut return_to = Some(&mut ret as *mut Litr);
  let mut scope = Scope::new(ScopeInner {
    parent:Some(f.scope),
    return_to:&mut return_to,
    class_defs:Vec::new(),
    class_uses:Vec::new(),
    kself,
    vars,
    module: this.module,
    outlives: Outlives::new()
  });
  scope.run(&f.stmts);
  ret
}

/// 调用本地函数，但会绑定自定义self
pub fn call_method(this:&Scope, f:&LocalFunc, kself:*mut Litr, args:Vec<Litr>)-> Litr {
  let mut vars = Vec::with_capacity(16);
  let mut args = args.into_iter();
  for (name,ty) in f.argdecl.iter() {
    let arg = args.next().unwrap_or(Litr::Uninit);
    vars.push((*name,arg))
  }

  let mut ret = Litr::Uninit;
  let mut return_to = Some(&mut ret as *mut Litr);
  let mut scope = Scope::new(ScopeInner {
    parent:Some(f.scope),
    return_to:&mut return_to,
    class_defs:Vec::new(),
    class_uses:Vec::new(),
    kself,
    vars,
    module: this.module,
    outlives: Outlives::new()
  });
  scope.run(&f.stmts);
  ret
}


/// 调用本地函数，但会绑定static处的module
pub fn call_static(this:&Scope, f:&LocalFunc, _module:*mut Module, args:Vec<Litr>)-> Litr {
  let mut vars = Vec::with_capacity(16);
  let mut args = args.into_iter();
  for (name,ty) in f.argdecl.iter() {
    let arg = args.next().unwrap_or(Litr::Uninit);
    vars.push((*name,arg))
  }

  let mut ret = Litr::Uninit;
  let mut return_to = Some(&mut ret as *mut Litr);
  let mut scope = Scope::new(ScopeInner {
    parent:Some(f.scope),
    return_to:&mut return_to,
    class_defs:Vec::new(),
    class_uses:Vec::new(),
    kself: this.kself,
    vars,
    module: f.scope.module,
    outlives: Outlives::new()
  });
  scope.run(&f.stmts);
  ret
}