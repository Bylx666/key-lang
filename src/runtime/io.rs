use crate::ast::{
  Expr,
  Stmt, Litr
};

macro_rules! args {($len:ident,$p:ident) => {
  unsafe{std::slice::from_raw_parts($p, $len)}
}}
pub extern fn print(len:usize, p: *const Litr)-> Litr {
  let args = args!(len, p);
  println!("{:?}", args);
  Litr::Uninit
  // if let Litr::Str(p) = p {
  //   println!("{}",p);
  // }
}