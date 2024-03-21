use crate::scan::literal::Litr;
use crate::runtime::{calc::CalcRef, Scope};

pub fn log(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  args.iter().for_each(|v|println!("{}", v.str()));
  Litr::Uninit
  // if let Litr::Str(p) = p {
  //   println!("{}",p);
  // }
}