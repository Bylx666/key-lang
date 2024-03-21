use crate::{
  intern::{intern, Interned}, 
  native::{BoundNativeMethod, NaitveInstanceRef, NativeFn}, 
  runtime::{calc::CalcRef, Scope}, 
  scan::{literal::{ArgDecl, Function, KsType, Litr, LocalFunc, LocalFuncRaw}, stmt::Statements}
};

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
