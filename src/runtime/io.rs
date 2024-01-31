use crate::ast::{
  Expr,
  Stmt, Litr
};

pub fn log(args:Vec<Litr>)-> Litr {
  println!("{:?}", args);
  Litr::Uninit
  // if let Litr::Str(p) = p {
  //   println!("{}",p);
  // }
}