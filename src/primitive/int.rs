
use crate::{intern::{intern, Interned}, native::NativeFn, runtime::{calc::CalcRef, Scope}};
use super::litr::Litr;

fn to_u32(n:&Litr)-> u32 {
  match n {
    Litr::Uint(n)=> *n as _,
    Litr::Int(n)=> *n as _,
    _=> 0
  }
}
fn to_isize(n:&Litr)-> isize {
  match n {
    Litr::Uint(n)=> *n as _,
    Litr::Int(n)=> *n,
    _=> 0
  }
}
fn to_usize(n:&Litr)-> usize {
  match n {
    Litr::Uint(n)=> *n,
    Litr::Int(n)=> *n as _,
    _=> 0
  }
}

pub fn method_int(n:isize, name:Interned, args:Vec<CalcRef>)-> Litr {
  macro_rules! get_arg0 {($deal:ident)=> {
    args.get(0).map_or(0, |n|$deal(n))
  }}
  match name.vec() {
    b"abs"=> Litr::Uint(n.unsigned_abs()),
    b"popcnt"=> Litr::Uint(n.count_ones() as _),
    b"rev"=> Litr::Int(n.swap_bytes()),
    b"leading"=> Litr::Uint(n.leading_zeros() as _),
    b"ending"=> Litr::Uint(n.trailing_zeros() as _),
    b"pow"=> Litr::Int(n.pow(get_arg0!(to_u32))),
    b"sqrt"=> Litr::Int(n.pow(2)),
    b"log"=> Litr::Uint(n.ilog(get_arg0!(to_isize)) as _),
    b"log2"=> Litr::Uint(n.ilog2() as _),
    b"log10"=> Litr::Uint(n.ilog10() as _),
    b"as_buf"=> Litr::Buf(n.to_ne_bytes().to_vec()),
    b"to_oct"=> Litr::Str(format!("{:o}", n)),
    b"to_str"=> Litr::Str(n.to_string()),
    b"to_hex"=> Litr::Str(format!("{:X}", n)),
    b"min"=> Litr::Int(n.min(get_arg0!(to_isize))),
    b"max"=> Litr::Int(n.max(get_arg0!(to_isize))),
    _=> panic!("{}上没有{}方法","int",name)
  }
}

pub fn method_uint(n:usize, name:Interned, args:Vec<CalcRef>)-> Litr {
  macro_rules! get_arg0 {($deal:ident)=> {
    args.get(0).map_or(0, |n|$deal(n))
  }}
  match name.vec() {
    b"popcnt"=> Litr::Uint(n.count_ones() as _),
    b"rev"=> Litr::Uint(n.swap_bytes()),
    b"leading"=> Litr::Uint(n.leading_zeros() as _),
    b"ending"=> Litr::Uint(n.trailing_zeros() as _),
    b"pow"=> Litr::Uint(n.pow(get_arg0!(to_u32))),
    b"sqrt"=> Litr::Uint(n.pow(2)),
    b"log"=> Litr::Uint(n.ilog(get_arg0!(to_usize)) as _),
    b"log2"=> Litr::Uint(n.ilog2() as _),
    b"log10"=> Litr::Uint(n.ilog10() as _),
    b"as_buf"=> Litr::Buf(n.to_ne_bytes().to_vec()),
    b"rotate_left"=> Litr::Uint(n.rotate_left(get_arg0!(to_u32))),
    b"rotate_right"=> Litr::Uint(n.rotate_right(get_arg0!(to_u32))),
    b"to_oct"=> Litr::Str(format!("{:o}", n)),
    b"to_str"=> Litr::Str(n.to_string()),
    b"to_hex"=> Litr::Str(format!("{:X}", n)),
    b"min"=> Litr::Uint(n.min(get_arg0!(to_usize))),
    b"max"=> Litr::Uint(n.max(get_arg0!(to_usize))),
    _=> panic!("{}上没有{}方法","int",name)
  }
}

// - statics int -
pub fn statics_int()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"parse"), s_parse_int),
    (intern(b"max"), |_,_|Litr::Int(isize::MAX)),
    (intern(b"min"), |_,_|Litr::Int(isize::MIN))
  ]
}

/// 根据传入的进制解析字符串
fn s_parse_int(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let radix = args.get(1).map_or(10, |n|to_u32(&**n));
  if let Some(s) = args.get(0) {
    return match &**s {
      Litr::Str(s)=> Litr::Int(isize::from_str_radix(s, radix)
        .unwrap_or_else(|_|panic!("Int::parse: 数字'{}'解析失败",s))),
      n=> Litr::Int(to_isize(n))
    }
  }
  Litr::Int(0)
}
