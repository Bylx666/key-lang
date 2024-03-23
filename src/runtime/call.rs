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
      _=> panic!("没有'{}'方法\n  如果你需要调用属性作为函数,请使用(a.b)()的写法", name)
    }
    // let mut left = self.calc_ref(left);
    // // 匹配所有可能使用.运算符得到函数的类型(instance, obj)
    // match &mut *left {
    //   Litr::Inst(inst)=> {
    //     let cannot_access_private = unsafe {(*inst.cls).module} != self.exports;
    //     let cls = unsafe {&*inst.cls};
  
    //     // 先找方法
    //     let methods = &cls.methods;
    //     for mthd in methods.iter() {
    //       if mthd.name == right {
    //         if !mthd.public && cannot_access_private {
    //           panic!("'{}'类型的成员方法'{}'是私有的", cls.name, right)
    //         }
    //         // 为函数绑定self
    //         let mut f = mthd.f.clone();
    //         f.bound = Some(Box::new(left));
    //         let args = args.into_iter().map(|v|v.own()).collect();
    //         return self.call_local(&f, args);
    //       }
    //     }
  
    //     // 再找属性
    //     let props = &cls.props;
    //     for (n, prop) in props.iter().enumerate() {
    //       if prop.name == right {
    //         if !prop.public && cannot_access_private {
    //           panic!("'{}'类型的成员属性'{}'是私有的", cls.name, right)
    //         }
    //         return self.call(args, CalcRef::Ref(&mut inst.v[n]));
    //       }
    //     }
    //   }
    //   Litr::Ninst(inst)=> {
    //     use crate::native::{NativeInstance, NaitveInstanceRef};
        
    //     let cls = unsafe{&*inst.cls};
    //     let inst: *mut NativeInstance = inst;
    //     let bound = match left {
    //       CalcRef::Own(v)=> if let Litr::Ninst(inst_own) = v {
    //         NaitveInstanceRef::Own(inst_own)
    //       }else {unreachable!()}
    //       CalcRef::Ref(_)=> NaitveInstanceRef::Ref(inst)
    //     };

    //     // 先找方法
    //     for (name, f) in cls.methods.iter() {
    //       if *name == right {
    //         return f(bound, args, self);
    //       }
    //     }

    //     // 再找属性
    //     return self.call(args, CalcRef::Own((cls.getter)(inst, right)));
    //   }
    //   Litr::Obj(map)=> return self.call(
    //     args, CalcRef::Ref(map.get_mut(&right).unwrap_or_else(||panic!("'{}'不是一个函数", right)))),
    //   Litr::Bool(v)=> panic!("Bool没有方法"),
    //   Litr::Buf(v)=> return primitive::buf::method(v, right, args),
    //   _=> ()
    // }
  }

  /// 实际调用一个local function
  pub fn call_local(self, f:&LocalFunc, args:Vec<Litr>)-> Litr {
    // 将传入参数按定义参数数量放入作用域
    let mut vars = Vec::with_capacity(16);
    let mut args = args.into_iter();
    for argdecl in f.argdecl.iter() {
      let arg = args.next().unwrap_or(argdecl.default.clone());
      vars.push((argdecl.name, arg));
    }
    // 如果函数被bound了就用bound值，否则继续沿用上级self
    let kself = self.kself;

    let mut ret = Litr::Uninit;
    let mut scope = Scope::new(ScopeInner {
      parent:Some(f.scope),
      return_to:&mut ret,
      class_defs:Vec::new(),
      class_uses:Vec::new(),
      kself,
      vars,
      imports: f.scope.imports,
      exports: f.scope.exports,
      outlives: AtomicUsize::new(0),
      ended: false
    });
    scope.run(&f.stmts);
    ret
  }
}
