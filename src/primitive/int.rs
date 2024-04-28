use super::litr::Litr;
use crate::{
    intern::{intern, Interned},
    native::NativeFn,
    runtime::{calc::CalcRef, Scope},
};

fn to_u32(n: &Litr) -> u32 {
    match n {
        Litr::Uint(n) => *n as _,
        Litr::Int(n) => *n as _,
        _ => 0,
    }
}
fn to_isize(n: &Litr) -> isize {
    match n {
        Litr::Uint(n) => *n as _,
        Litr::Int(n) => *n,
        _ => 0,
    }
}
fn to_usize(n: &Litr) -> usize {
    match n {
        Litr::Uint(n) => *n,
        Litr::Int(n) => *n as _,
        _ => 0,
    }
}

pub fn method_int(n: isize, name: Interned, args: Vec<CalcRef>) -> Litr {
    macro_rules! get_arg0 {
        ($deal:ident) => {
            args.first().map_or(0, |n| $deal(n))
        };
    }
    match name.vec() {
        b"pow" => args.first().map_or(Litr::Int(1), |val| match &**val {
            Litr::Uint(r) => Litr::Int(n.pow(*r as _)),
            Litr::Int(r) => Litr::Int(n.pow(*r as _)),
            Litr::Float(r) => Litr::Float((n as f64).powf(*r)),
            _ => Litr::Int(1),
        }),
        b"log" => Litr::Uint(n.ilog(get_arg0!(to_isize)) as _),
        b"log2" => Litr::Uint(n.ilog2() as _),
        b"log10" => Litr::Uint(n.ilog10() as _),

        b"abs" => Litr::Uint(n.unsigned_abs()),
        b"as_buf" => Litr::Buf(n.to_ne_bytes().to_vec()),
        b"to_str" => Litr::Str(n.to_string()),
        b"to_oct" => Litr::Str(format!("{:o}", n)),
        b"to_hex" => Litr::Str(format!("{:X}", n)),

        b"min" => Litr::Int(n.min(get_arg0!(to_isize))),
        b"max" => Litr::Int(n.max(get_arg0!(to_isize))),
        b"rev" => Litr::Int(n.swap_bytes()),
        _ => panic!("{}上没有{}方法", "Int", name),
    }
}

pub fn method_uint(n: usize, name: Interned, args: Vec<CalcRef>) -> Litr {
    macro_rules! get_arg0 {
        ($deal:ident) => {
            args.first().map_or(0, |n| $deal(n))
        };
    }
    match name.vec() {
        b"pow" => args.first().map_or(Litr::Uint(1), |val| match &**val {
            Litr::Uint(r) => Litr::Uint(n.pow(*r as _)),
            Litr::Int(r) => Litr::Uint(n.pow(*r as _)),
            Litr::Float(r) => Litr::Float((n as f64).powf(*r)),
            _ => Litr::Uint(1),
        }),
        b"log" => Litr::Uint(n.ilog(get_arg0!(to_usize)) as _),
        b"log2" => Litr::Uint(n.ilog2() as _),
        b"log10" => Litr::Uint(n.ilog10() as _),

        b"as_buf" => Litr::Buf(n.to_ne_bytes().to_vec()),
        b"as8" => Litr::Buf(vec![n as u8]),
        b"as16" => Litr::Buf(n.to_ne_bytes().to_vec()),
        b"as32" => Litr::Buf(n.to_ne_bytes().to_vec()),
        b"as64" => Litr::Buf(n.to_ne_bytes().to_vec()),
        b"to_oct" => Litr::Str(format!("{:o}", n)),
        b"to_str" => Litr::Str(n.to_string()),
        b"to_hex" => Litr::Str(format!("{:X}", n)),

        // bin
        b"popcnt" => Litr::Uint(n.count_ones() as _),
        b"rev" => Litr::Uint(n.swap_bytes()),
        b"leading" => Litr::Uint(n.leading_zeros() as _),
        b"ending" => Litr::Uint(n.trailing_zeros() as _),
        b"rotate_left" => Litr::Uint(n.rotate_left(get_arg0!(to_u32))),
        b"rotate_right" => Litr::Uint(n.rotate_right(get_arg0!(to_u32))),

        b"min" => Litr::Uint(n.min(get_arg0!(to_usize))),
        b"max" => Litr::Uint(n.max(get_arg0!(to_usize))),
        _ => panic!("{}上没有{}方法", "Uint", name),
    }
}

// - statics int -
pub fn statics_int() -> Vec<(Interned, NativeFn)> {
    vec![
        (intern(b"parse"), s_parse_int),
        (intern(b"max"), |_, _| Litr::Int(isize::MAX)),
        (intern(b"min"), |_, _| Litr::Int(isize::MIN)),
    ]
}

/// 根据传入的进制解析字符串
fn s_parse_int(args: Vec<CalcRef>, _cx: Scope) -> Litr {
    let radix = args.get(1).map_or(10, |n| to_u32(n));
    if let Some(s) = args.first() {
        return match &**s {
            Litr::Str(s) => Litr::Int(
                isize::from_str_radix(s, radix)
                    .unwrap_or_else(|_| panic!("Int::parse: 数字'{}'解析失败", s)),
            ),
            n => Litr::Int(to_isize(n)),
        };
    }
    Litr::Int(0)
}

// - statics uint -
pub fn statics_uint() -> Vec<(Interned, NativeFn)> {
    vec![
        (intern(b"parse"), s_parse_uint),
        (intern(b"max"), |_, _| Litr::Uint(usize::MAX)),
        (intern(b"min"), |_, _| Litr::Uint(usize::MIN)),
    ]
}

/// 根据传入的进制解析字符串
fn s_parse_uint(args: Vec<CalcRef>, _cx: Scope) -> Litr {
    let radix = args.get(1).map_or(10, |n| to_u32(n));
    if let Some(s) = args.first() {
        return match &**s {
            Litr::Str(s) => Litr::Uint(
                usize::from_str_radix(s, radix)
                    .unwrap_or_else(|_| panic!("Uint::parse: 数字'{}'解析失败", s)),
            ),
            n => Litr::Uint(to_usize(n)),
        };
    }
    Litr::Uint(0)
}
