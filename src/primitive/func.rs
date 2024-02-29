use crate::{
  intern::{intern, Interned}, 
  native::NativeFn, 
  runtime::calc::CalcRef, 
  scan::literal::Litr
};

fn s_new(s:Vec<CalcRef>)-> Litr {
  Litr::Bool(false)
}
