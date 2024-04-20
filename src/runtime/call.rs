use crate::primitive;

use self::calc::CalcRef;

use super::*;

impl Scope {
  /// 解析Expr的调用
  pub fn call(mut self, args:Vec<CalcRef>, targ:&Function)-> Litr {
    use Function::*;
    match targ {
      Native(f)=> f(args, self),
      Local(f)=> {
        let args = args.into_iter().map(|e|e.own()).collect();
        self.call_local(&f, args)
      },
      Extern(f)=> {
        let args = args.into_iter().map(|e|e.own()).collect();
        super::externer::call_extern(&f, args)
      }
    }
  }

  /// 为a.b()的行为匹配对应方法并调用
  pub fn call_method(mut self, mut args:Vec<CalcRef>, mut targ:CalcRef, name:Interned)-> Litr {
    match &mut *targ {
      Litr::Bool(v)=> match name.vec() {
        b"rev"=> Litr::Bool(!*v),
        b"then"=> {
          let f = match args.get_mut(0) {
            Some(f)=> match &**f {
              Litr::Func(f)=> f,
              _=> panic!("bool.then第一个参数必须是函数")
            },
            None=> return Litr::Uninit
          };
          if *v {
            self.call(vec![], f)
          }else {
            Litr::Uninit
          }
        }
        _=> panic!("Bool类型只有'rev'和'then'方法")
      }
      Litr::Buf(v)=> primitive::buf::method(v, self, name, args),
      Litr::List(v)=> primitive::list::method(v, self, name, args),
      Litr::Obj(o)=> primitive::obj::method(o, self, name, args),
      Litr::Int(n)=> primitive::int::method_int(*n, name, args),
      Litr::Uint(n)=> primitive::int::method_uint(*n, name, args),
      Litr::Float(n)=> primitive::float::method(*n, name, args),
      Litr::Str(s)=> primitive::kstr::method(s, self, name, args),
      Litr::Func(f)=> primitive::func::method(f, name, self, args),
      Litr::Uninit=> panic!("uninit没有方法"),
      Litr::Inst(inst)=> {
        let cannot_access_private = unsafe {(*inst.cls).cx.exports} != self.exports;
        let cls = unsafe {&*inst.cls};

        let methods = &cls.methods;
        for mthd in methods.iter() {
          if mthd.name == name {
            if !mthd.public && cannot_access_private {
              panic!("'{}'类型的成员方法'{}'是私有的", cls.name, name)
            }
            let mut f = LocalFunc::new(&mthd.f, cls.cx);
            let args = args.into_iter().map(|e|e.own()).collect();
            return Scope::call_local_with_self(&f, args, &mut *targ);
          }
        }

        panic!("'{}'类型没有'{}'方法\n  你需要用(x.{})()的写法吗?",cls.name, name, name)
      }
      Litr::Ninst(inst)=> {
        let cls = unsafe{&*inst.cls};
        let (_,f) = cls.methods.iter()
          .find(|(find,_)|name==*find).unwrap_or_else(||panic!("'{}'原生类型中没有'{}'方法\n  你需要用(x.{})()的写法吗?", cls.name, name, name));
        (*f)(inst, args, self)
      }
    }
  }

  /// 实际调用一个local function
  pub fn call_local(self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    Scope::call_local_with_self(f, args, self.kself)
  }
  
  /// 实际调用一个local function并传入self
  pub fn call_local_with_self(f:&LocalFunc, args:Vec<Litr>, kself:*mut Litr)-> Litr {
    // 将传入参数按定义参数数量放入作用域
    let init_vars = match &f.argdecl {
      /// 正常传参
      LocalFuncRawArg::Normal(argdecl)=> {
        // 将传入参数按定义参数数量放入作用域
        let mut vars = Vec::with_capacity(f.stmts.vars + argdecl.len());
        let mut args = args.into_iter();
        for argdecl in argdecl.iter() {
          let mut arg = args.next().unwrap_or_else(||f.scope.calc(&argdecl.default));
          // 将传的空参数转为默认参数
          if let Litr::Uninit = arg {
            arg = f.scope.calc(&argdecl.default);
          }
          assert!(argdecl.t.is(&arg, f.scope), "函数要求{:?}类型, 但传入了{:?}", argdecl.t, arg);
          let var = Variant {name:argdecl.name, v:arg, locked:false};
          vars.push(var);
        }
        vars
      }
      /// List传参
      LocalFuncRawArg::Custom(name)=> {
        let mut vars = Vec::with_capacity(f.stmts.vars + 1);
        vars.push(Variant {name:*name, v:Litr::List(args), locked:false});
        vars
      }
    };

    let mut ret = Litr::Uninit;
    let mut scope = f.scope.subscope();
    scope.return_to = &mut ret;
    scope.vars = init_vars;
    scope.kself = kself;
    scope.run(&f.stmts);
    ret
  }
}
