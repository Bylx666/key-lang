//! Obj基本类型的静态方法

use crate::{
  intern::{intern, Interned}, 
  native::NativeFn, 
  runtime::{calc::CalcRef, Scope},
  primitive::litr::Litr
};
use std::collections::HashMap;

pub fn statics()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"insert"), s_insert),
    (intern(b"remove"), s_remove),
    (intern(b"has"), s_has),
    (intern(b"get"), s_get),
  ]
}

/// static insert
fn s_insert(mut args:Vec<CalcRef>, _cx:Scope)-> Litr {
  assert!(args.len()>=3, "Obj::insert需要3个参数: obj, name:Str, val");
  let mut args = args.iter_mut();
  let targ = match &mut**args.next().unwrap() {
    Litr::Obj(m)=> m,
    _=> panic!("insert第一个参数必须是Obj")
  };
  let id = match &mut **args.next().unwrap() {
    Litr::Str(s)=> s,
    _=> panic!("insert第二个参数必须是Str")
  };
  let v = args.next().unwrap().clone();
  match targ.insert(intern(id.as_bytes()), v.own()) {
    Some(v)=> v,
    None=> Litr::Uninit
  }
}

/// static remove
fn s_remove(mut args:Vec<CalcRef>, _cx:Scope)-> Litr {
  assert!(args.len()>=2, "Obj::remove需要2个参数: obj, name:Str");
  let mut args = args.iter_mut();
  let targ = match &mut**args.next().unwrap() {
    Litr::Obj(m)=> m,
    _=> panic!("remove第一个参数必须是Obj")
  };
  let id = match &mut **args.next().unwrap() {
    Litr::Str(s)=> s,
    _=> panic!("remove第二个参数必须是Str")
  };
  match targ.remove(&intern(id.as_bytes())) {
    Some(v)=> v,
    None=> Litr::Uninit
  }
}

/// static has
fn s_has(mut args:Vec<CalcRef>, _cx:Scope)-> Litr {
  assert!(args.len()>=2, "Obj::has需要2个参数: obj, name:Str");
  let mut args = args.iter_mut();
  let targ = match &mut**args.next().unwrap() {
    Litr::Obj(m)=> m,
    _=> panic!("has第一个参数必须是Obj")
  };
  let id = match &mut **args.next().unwrap() {
    Litr::Str(s)=> s,
    _=> panic!("has第二个参数必须是Str")
  };
  match targ.get(&intern(id.as_bytes())) {
    Some(_)=> Litr::Bool(true),
    None=> Litr::Bool(false)
  }
}

/// static get
fn s_get(mut args:Vec<CalcRef>, _cx:Scope)-> Litr {
  assert!(args.len()>=2, "Obj::has需要2个参数: obj, name:Str");
  let mut args = args.iter_mut();
  let targ = match &mut**args.next().unwrap() {
    Litr::Obj(m)=> m,
    _=> panic!("has第一个参数必须是Obj")
  };
  let id = match &mut **args.next().unwrap() {
    Litr::Str(s)=> s,
    _=> panic!("has第二个参数必须是Str")
  };
  match targ.get(&intern(id.as_bytes())) {
    Some(v)=> v.clone(),
    None=> Litr::Uninit
  }
}
