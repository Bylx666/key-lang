use crate::scan::literal::Litr;

pub fn log(args:Vec<Litr>)-> Litr {
  println!("{}", args[0].str());
  Litr::Uninit
  // if let Litr::Str(p) = p {
  //   println!("{}",p);
  // }
}