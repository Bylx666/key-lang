use crate::{
  runtime::{calc::CalcRef, Scope}, 
  primitive::litr::Litr,
  intern::{Interned, intern},
  native::NativeFn
};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Symbol {
  IterEnd,
  Reserved
}


pub fn statics()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"iter_end"), s_iter_end)
  ]
}

fn s_iter_end(_:Vec<CalcRef>, _cx:Scope)-> Litr {
  Litr::Sym(Symbol::IterEnd)
}