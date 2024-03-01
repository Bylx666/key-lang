use super::*;

pub fn method(v:&mut Vec<u8>, name:Interned, args:Vec<CalcRef>)-> Litr {
  match name.vec() {
    b"splice"=> splice(v, args),
    _=> err!("s")
  }
}

pub fn splice(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr { 
  let mut args = args.into_iter();
  let arg0 = match args.next() {
    Some(v)=> v,
    None=> err!("splice方法至少提供一个参数")
  };
  Litr::Uninit
}