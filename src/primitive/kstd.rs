use crate::intern::{Interned,intern};
use crate::primitive::litr::{Litr, Function};
use crate::runtime::{calc::CalcRef, Scope};

pub fn prelude()-> Vec<(Interned, Litr)> {
  macro_rules! prel {($($name:literal:$f:ident)*)=>{
    vec![$( (intern($name),Litr::Func(Function::Native($f))), )*]
  }}
  prel!{
    b"log":log
    b"evil":evil
    // b"swap":
  }
}

/// 输出到控制台
pub fn log(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  args.iter().for_each(|v|println!("{}", v.str()));
  Litr::Uninit
  // if let Litr::Str(p) = p {
  //   println!("{}",p);
  // }
}

/// 在当前作用域 解析并运行一段String
pub fn evil(args:Vec<CalcRef>, cx:Scope)-> Litr {
  let s = args.get(0).expect("evil需要传入一个被解析的字符串或数组");
  let s = match &**s {
    Litr::Str(s)=> s.as_bytes(),
    Litr::Buf(b)=> &**b,
    _=> panic!("evil只能运行字符串或数组")
  };
  let scanned = crate::scan::scan(s);
  println!("{scanned:?}");
  cx.run(&scanned);
  Litr::Uninit
}

