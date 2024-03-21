use super::*;

pub fn method(v:&mut Vec<u8>, name:Interned, args:Vec<CalcRef>)-> Litr {
  match name.vec() {
    b"push"=> push(v, args),
    b"splice"=> splice(v, args),
    _=> err!("Buf没有{}方法",name)
  }
}

const fn to_u8(v:&Litr)-> u8 {
  match v {
    Litr::Int(n)=> *n as u8,
    Litr::Uint(n)=> *n as u8,
    _=> 0
  }
}

pub fn push(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.into_iter();
  match &*next_arg!(args "'push'方法需要一个数字,列表或数组作为参数") {
    Litr::Buf(right)=> v.extend_from_slice(right),
    Litr::List(right)=> v.extend_from_slice(
      &right.iter().map(|litr|to_u8(litr)).collect::<Vec<u8>>()),
    n=> v.push(to_u8(n))
  };
  Litr::Uninit
}

fn splice(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr { 
  let mut args = args.into_iter();
  let arg0 = next_arg!(args "splice方法至少提供一个参数");
  Litr::Uninit
}
