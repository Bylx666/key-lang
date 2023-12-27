use crate::ast::{
  Expr,
  Exprp,
  Statmnt, Imme
};
pub fn print(p: &Vec<Imme>) {
  if p.len() > 0 {
    match &p[0] {
      Imme::Str(s)=> {
        println!("{}", String::from_utf8_lossy(&s));
      }
      _=> {}
    }
  }
}