use crate::{
  intern::{intern, Interned}, 
  native::{BoundNativeMethod, NativeFn}, 
  runtime::{calc::CalcRef, err, Scope}, 
  scan::{literal::{ArgDecl, Function, KsType, Litr, LocalFunc, LocalFuncRaw}, stmt::Statements}
};

pub fn statics()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"new"), s_new)
  ]
}

fn s_new(s:Vec<CalcRef>, cx:Scope)-> Litr {
  let mut itr = s.into_iter();
  let arg1 = itr.next();
  let stmts = match &arg1 {
    Some(arg)=> crate::scan::scan(match &**arg {
      Litr::Str(s)=> s.as_bytes(),
      Litr::Buf(b)=> b,
      _=> err!("Func::new第一个参数必须是Str或Buf, 用来被解析为函数体")
    }),
    None=> Statements(Vec::new())
  };

  let arg2 = itr.next();
  let argdecl = match &arg2 {
    Some(arg)=> match &**arg {
      Litr::List(v)=> 
        v.iter().map(|v|match v {
          Litr::Str(s)=> ArgDecl {default: Litr::Uninit, name: intern(s.as_bytes()), t:KsType::Any},
          _=> ArgDecl {default: Litr::Uninit, name: intern(b"#ignored"), t:KsType::Any}
        }).collect(),
      _=> err!("Func::new第二个参数必须是Str组成的List, 用来定义该函数的参数名")
    },
    None=> Vec::new()
  };
  
  Litr::Func(Function::Local(LocalFunc::new(Box::into_raw(Box::new(
    LocalFuncRaw {argdecl, stmts}
  )),cx)))
}

// pub fn prop(s:CalcRef, p:Interned)-> Litr {
//   Litr::Func(Function::NativeMethod(BoundNativeMethod {f: match p.vec() {
//     b"call_here"=> call_here,
//     _=> err!("Func没有{}属性", p)
//   }, bind: s}))
// }

fn call_here(v:Litr, args:Vec<CalcRef>)-> Litr {
  Litr::Uninit
}
