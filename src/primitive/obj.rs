//! Obj基本类型的静态方法

use crate::{intern::Interned, native::NativeFn, runtime::err, scan::literal::Litr};
use std::collections::HashMap;

// pub fn get_impl(s:Interned)-> Option<NativeFn> {
//   match s.vec() {
//     b"insert"=> 
//     b"remove"=> {}
//   }
// }

// /// static insert
// fn s_insert(args:Vec<Litr>)-> Litr {
//   if args.len()<3 {err!("Obj::insert需要3个参数: obj, name:Str, val")};
//   let targ = match args[0] {
//     Litr::Obj(m)=> &*m,
//     _=> err!("insert第一个参数必须是Obj")
//   };
  
// }
