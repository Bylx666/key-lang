use crate::ast::{
  Expr,
  Stmt, Litr
};
pub fn print(p: Vec<Litr>)-> Litr {
  println!("{:?}", p);
  Litr::Uninit
  // if let Litr::Str(p) = p {
  //   println!("{}",p);
  // }
}