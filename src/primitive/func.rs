use crate::{
  intern::{intern, Interned}, 
  native::NativeFn, 
  runtime::{calc::CalcRef, Scope}, 
  primitive::litr::{ArgDecl, Function, KsType, Litr, LocalFunc, LocalFuncRaw}, 
  scan::stmt::Statements
};

pub fn method(f:&Function, name:Interned, cx: Scope, args:Vec<CalcRef>)-> Litr {
  match name.vec() {
    b"call"=> kcall(f, args, cx),
    b"clone_here"=> clone_here(f, args, cx),
    b"call_here"=> call_here(f, args, cx),
    b"clone_top"=> clone_top(f, args, cx),
    _=> panic!("func没有{}方法",name)
  }
}

/// 传入self并调用
pub fn kcall(f:&Function, mut args:Vec<CalcRef>, mut cx:Scope)-> Litr {
  assert!(args.len()>=1, "func.call必须传入一个值作为self");
  let trans_args = args.split_off(1);
  let mut kself = args.pop().unwrap();
  match f {
    Function::Local(f)=> cx.call_local_with_self(
      f, 
      trans_args.into_iter().map(|v|v.own()).collect(), 
      &mut *kself
    ),
    // 如果不是local就正常调用
    _=> cx.call(trans_args, f)
  }
}

/// 复制一个函数,但上下文在当前作用域
pub fn clone_here(f:&Function, mut args:Vec<CalcRef>, mut cx:Scope)-> Litr {
  Litr::Func(match f {
    Function::Local(f)=> Function::Local(LocalFunc::new(f.ptr, cx)),
    // 如果不是local就正常调用
    _=> f.clone()
  })
}

/// 复制一个函数,但上下文在当前作用域
pub fn call_here(f:&Function, mut args:Vec<CalcRef>, mut cx:Scope)-> Litr {
  assert!(args.len()>=1, "func.call_here必须传入一个值作为self");
  let trans_args = args.split_off(1);
  let mut kself = args.pop().unwrap();
  match f {
    Function::Local(f)=> cx.call_local_with_self(
      &LocalFunc::new(f.ptr, cx), 
      trans_args.into_iter().map(|v|v.own()).collect(), 
      &mut *kself
    ),
    // 如果不是local就正常调用
    _=> cx.call(trans_args, f)
  }
}

/// 复制一个函数,但上下文在该模块的顶级作用域
pub fn clone_top(f:&Function, mut args:Vec<CalcRef>, mut cx:Scope)-> Litr {
  // 获取顶级作用域
  while let Some(s) = &cx.parent {
    cx = s.clone()
  }
  Litr::Func(match f {
    Function::Local(f)=> Function::Local(LocalFunc::new(f.ptr, cx)),
    // 如果不是local就正常调用
    _=> f.clone()
  })
}

pub fn statics()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"new"), s_new)
  ]
}

fn s_new(mut s:Vec<CalcRef>, cx:Scope)-> Litr {
  let mut itr = s.iter_mut();
  let stmts = match itr.next() {
    Some(arg)=> crate::scan::scan(match &**arg {
      Litr::Str(s)=> s.as_bytes(),
      Litr::Buf(b)=> b,
      _=> panic!("Func::new第一个参数必须是Str或Buf, 用来被解析为函数体")
    }),
    None=> Statements(Vec::new())
  };

  let argdecl = match itr.next() {
    Some(arg)=> match &**arg {
      Litr::List(v)=> 
        v.iter().map(|v|match v {
          Litr::Str(s)=> ArgDecl {default: Litr::Uninit, name: intern(s.as_bytes()), t:KsType::Any},
          _=> ArgDecl {default: Litr::Uninit, name: intern(b"#ignored"), t:KsType::Any}
        }).collect(),
      _=> panic!("Func::new第二个参数必须是Str组成的List, 用来定义该函数的参数名")
    },
    None=> Vec::new()
  };
  
  Litr::Func(Function::Local(LocalFunc::new(Box::into_raw(Box::new(
    LocalFuncRaw {argdecl, stmts}
  )),cx)))
}
