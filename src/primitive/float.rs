use super::litr::Litr;
use crate::{
    intern::{intern, Interned},
    native::NativeFn,
    runtime::{calc::CalcRef, Scope},
};

fn to_f(v: &Litr) -> f64 {
    match v {
        Litr::Uint(n) => *n as _,
        Litr::Int(n) => *n as _,
        Litr::Float(n) => *n,
        _ => 0.0,
    }
}
pub fn method(n: f64, name: Interned, args: Vec<CalcRef>) -> Litr {
    macro_rules! get_arg0 {
        ($deal:ident) => {
            args.first().map_or(0.0, |n| $deal(n))
        };
    }
    match name.vec() {
        // exponential 指数
        b"log" => Litr::Float(n.log(get_arg0!(to_f))),
        b"log2" => Litr::Float(n.log2()),
        b"log10" => Litr::Float(n.log10()),
        b"ln" => Litr::Float(n.ln()),
        b"log1p" => Litr::Float(n.ln_1p()),
        b"exp" => Litr::Float(n.exp()),
        b"exp2" => Litr::Float(n.exp2()),
        b"expm1" => Litr::Float(n.exp_m1()),
        b"pow" => Litr::Float(args.first().map_or(0.0, |val| match &**val {
            Litr::Uint(r) => n.powi(*r as _),
            Litr::Int(r) => n.powi(*r as _),
            Litr::Float(r) => n.powf(*r),
            _ => 1.0,
        })),
        b"sqrt" => Litr::Float(n.sqrt()),
        b"cbrt" => Litr::Float(n.cbrt()),
        b"recip" => Litr::Float(n.recip()),
        b"hypot" => Litr::Float(n.hypot(get_arg0!(to_f))),

        // triangles 三角函数
        b"acos" => Litr::Float(n.acos()),
        b"acosh" => Litr::Float(n.acosh()),
        b"asin" => Litr::Float(n.asin()),
        b"asinh" => Litr::Float(n.asinh()),
        b"atan" => Litr::Float(n.atan()),
        b"atan2" => Litr::Float(n.atan2(get_arg0!(to_f))),
        b"atanh" => Litr::Float(n.atanh()),
        b"cos" => Litr::Float(n.cos()),
        b"cosh" => Litr::Float(n.cosh()),
        b"sin" => Litr::Float(n.sin()),
        b"sinh" => Litr::Float(n.sinh()),
        b"tan" => Litr::Float(n.tan()),
        b"tanh" => Litr::Float(n.tanh()),
        b"sincos" => Litr::List(vec![Litr::Float(n.sin()), Litr::Float(n.cos())]),

        // rounding 四舍五入
        b"ceil" => Litr::Int(n.ceil() as _),
        b"floor" => Litr::Int(n.floor() as _),
        b"round" => Litr::Int(n.round() as _),
        b"trunc" => Litr::Int(n.trunc() as _),

        // compare 比较
        b"max" => Litr::Float(n.max(get_arg0!(to_f))),
        b"min" => Litr::Float(n.min(get_arg0!(to_f))),
        b"clamp" => Litr::Float({
            assert!(args.len() >= 2, "float.clamp需要2个Float作为参数");
            let [mut min, mut max] = [to_f(args.first().unwrap()), to_f(args.get(1).unwrap())];
            if min > max {
                std::mem::swap(&mut min, &mut max);
            }
            n.clamp(min, max)
        }),

        // sign 符号 注意0和-0的符号不一样
        b"abs" => Litr::Float(n.abs()),
        b"copy_sign" => Litr::Float(n.copysign(get_arg0!(to_f))),
        b"is_pos" => Litr::Bool(n.is_sign_positive()),

        // Pi 圆周
        b"deg" => Litr::Float(n.to_degrees()),
        b"rad" => Litr::Float(n.to_radians()),

        // memory 内存
        b"as_buf" => Litr::Buf(n.to_ne_bytes().to_vec()),
        b"rev" => Litr::Float({
            let mut b = n.to_ne_bytes();
            b.reverse();
            // SAFETY: 这十分unsafe, 不过在考虑字节序时这会很便利
            unsafe { std::mem::transmute::<[u8; 8], f64>(b) }
        }),
        b"is_nan" => Litr::Bool(n.is_nan()),
        b"is_inf" => Litr::Bool(n.is_infinite()),

        // string 字符
        b"to_str" => Litr::Str(n.to_string()),
        b"to_fixed" => Litr::Str({
            match args.first() {
                Some(fix_to) => {
                    let mut s = n.to_string();

                    let fix_to = 2 + match &**fix_to {
                        Litr::Int(n) => *n as _,
                        Litr::Uint(n) => *n as _,
                        _ => 0,
                    };
                    let trunc_len = n.trunc().to_string().len();
                    let len = trunc_len - 1 + fix_to;

                    if len >= s.len() {
                        s.push_str(&"0".repeat(len - s.len()));
                    } else {
                        s.truncate(len);
                    }
                    s
                }
                None => n.to_string(),
            }
        }),

        _ => panic!("{}上没有{}方法", "Float", name),
    }
}

// - statics -
pub fn statics() -> Vec<(Interned, NativeFn)> {
    use std::f64::consts;
    vec![
        (intern(b"parse"), s_parse),
        (intern(b"ln2"), |_, _| Litr::Float(consts::LN_2)),
        (intern(b"ln10"), |_, _| Litr::Float(consts::LN_10)),
        (intern(b"log10_e"), |_, _| Litr::Float(consts::LOG10_E)),
        (intern(b"log10_2"), |_, _| Litr::Float(consts::LOG10_2)),
        (intern(b"log2_10"), |_, _| Litr::Float(consts::LOG2_10)),
        (intern(b"log2_e"), |_, _| Litr::Float(consts::LOG2_E)),
        (intern(b"sqrt2"), |_, _| Litr::Float(consts::SQRT_2)),
        (intern(b"sqrt1_2"), |_, _| {
            Litr::Float(consts::FRAC_1_SQRT_2)
        }),
        (intern(b"frac1_pi"), |_, _| Litr::Float(consts::FRAC_1_PI)),
        (intern(b"pi"), |_, _| Litr::Float(consts::PI)),
        (intern(b"tau"), |_, _| Litr::Float(consts::TAU)),
        (intern(b"e"), |_, _| Litr::Float(consts::E)),
        (intern(b"max"), |_, _| Litr::Float(f64::MAX)),
        (intern(b"min"), |_, _| Litr::Float(f64::MIN)),
        (intern(b"nan"), |_, _| Litr::Float(f64::NAN)),
        (intern(b"inf"), |_, _| Litr::Float(f64::INFINITY)),
        (intern(b"neg_inf"), |_, _| Litr::Float(f64::NEG_INFINITY)),
    ]
}

/// 解析浮点数
fn s_parse(args: Vec<CalcRef>, _cx: Scope) -> Litr {
    if let Some(s) = args.first() {
        return match &**s {
            Litr::Str(s) => Litr::Float(
                <f64 as std::str::FromStr>::from_str(s)
                    .unwrap_or_else(|_| panic!("Float::parse: 数字'{}'解析失败", s)),
            ),
            n => Litr::Float(to_f(n)),
        };
    }
    Litr::Float(0.0)
}
