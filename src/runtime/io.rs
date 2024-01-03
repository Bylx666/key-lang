use crate::ast::{
  Expr,
  Stmt, Litr
};
pub fn print(p: &Litr) {
  println!("{:?}", p);
  // if p.len() > 0 {
  //   match &p[0] {
  //     Litr::Str(s)=> {
  //       println!("{}", String::from_utf8_lossy(&s));
  //     }
  //     _=> {}
  //   }
  // }
}