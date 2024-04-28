//! key的str实现

use super::*;

fn to_usize(n: &Litr) -> usize {
    match n {
        Litr::Uint(n) => *n,
        Litr::Int(n) => *n as _,
        _ => 0,
    }
}

static mut ITER_LINES: *mut NativeClassDef = std::ptr::null_mut();

pub fn method(s: &mut String, scope: Scope, name: Interned, args: Vec<CalcRef>) -> Litr {
    macro_rules! get_arg0 {
        // 解析为usize
        (usize) => {
            args.first().map_or(0, |n| to_usize(n))
        };
        // 解析为该字符索引
        (index) => {{
            let n = get_arg0!(usize);
            s.char_indices()
                .nth(n)
                .unwrap_or_else(|| panic!("索引{}超出字符范围", n))
                .0
        }};
        // 解析为字符
        (str $err:literal) => {
            match &**args
                .first()
                .unwrap_or_else(|| panic!("str.{}第一个参数必须是Str", $err))
            {
                Litr::Str(s) => s,
                _ => panic!("str.{}第一个参数必须是Str", $err),
            }
        };
    }

    /// 直接返回Litr::Uninit, 用小括号去减少花括号
    macro_rules! void {
        ($v:expr) => {{
            $v;
            Litr::Uninit
        }};
    }

    match name.vec() {
        // search
        b"includes" => Litr::Bool(s.contains(get_arg0!(str "includes"))),
        b"start_with" => Litr::Bool(s.starts_with(get_arg0!(str "start_with"))),
        b"ends_with" => Litr::Bool(s.ends_with(get_arg0!(str "ends_with"))),
        b"index_of" => index_of(s, args),
        b"r_index_of" => r_index_of(s, args),

        // edit
        b"cut" => Litr::Str(s.split_off(get_arg0!(index))),
        b"insert" => insert(s, args),
        b"remove" => remove(s, args),
        b"push" => void!(s.push_str(get_arg0!(str "push"))),
        b"slice" => Litr::Str(_slice(s, args)),
        b"to_lcase" => Litr::Str({
            let mut s = s.clone();
            s.make_ascii_lowercase();
            s
        }),
        b"to_ucase" => Litr::Str({
            let mut s = s.clone();
            s.make_ascii_lowercase();
            s
        }),
        b"rev" => Litr::Str(s.chars().rev().collect()),
        b"repeat" => Litr::Str(s.repeat(get_arg0!(usize))),
        b"replace" => Litr::Str(_replace(s, args)),
        b"splice" => splice(s, args),
        b"trim" => Litr::Str(s.trim().to_string()),

        // transmute
        b"to_buf" => Litr::Buf(s.as_bytes().to_vec()),
        b"to_utf16" => to_utf16(s),
        b"split" => split(s, args),

        b"case_eq" => Litr::Bool(s.eq_ignore_ascii_case(get_arg0!(str "englisheq"))),
        b"lines" => lines(s),

        _ => panic!("Str上没有{}方法", name),
    }
}

macro_rules! _index_of {
    ($s:ident,$args:ident,$id:ident) => {{
        let find = match &**$args.first().expect("str.index_of需要知道你找的字符串") {
            Litr::Str(s) => s,
            _ => panic!("str.index_of第一个参数必须是Str",),
        };
        Litr::Int(match $s.$id(find) {
            // 将byte index转为char index
            Some(end) => {
                let mut i = 0;
                let mut iter = $s.char_indices();
                while let Some((now_i, _)) = iter.next() {
                    if now_i >= end {
                        break;
                    }
                    i += 1;
                }
                i as isize
            }
            None => -1,
        })
    }};
}

/// 寻找第一个该字符的索引
fn index_of(s: &mut str, args: Vec<CalcRef>) -> Litr {
    _index_of!(s, args, find)
}

/// 寻找倒数第一个该字符的索引
fn r_index_of(s: &mut str, args: Vec<CalcRef>) -> Litr {
    _index_of!(s, args, rfind)
}

/// 删除一个(只传1个索引)或删除多个(并传了删除字数)字符
///
/// 可以不传参数,直接当pop使用
fn remove(s: &mut String, args: Vec<CalcRef>) -> Litr {
    let mut indice = s.char_indices();
    let index = args.first().map_or(0, |n| to_usize(n));
    let index = indice.nth(index).map_or(s.len(), |(n, _)| n);
    // 可传入一个删除长度(字符长度,不是字节长度)
    Litr::Str(if let Some(len) = args.get(1) {
        let len = to_usize(len) - 1;
        let len = indice.nth(len).unwrap_or((s.len(), '\x00')).0;
        s.drain(index..len).collect()
    } else {
        s.remove(index).to_string()
    })
}

/// 在索引处插入一段字符
fn insert(s: &mut String, args: Vec<CalcRef>) -> Litr {
    assert!(args.len() >= 2, "str.insert必须传入两个参数");
    let mut indice = s.char_indices();
    let index = indice
        .nth(to_usize(args.first().unwrap()))
        .unwrap_or_else(|| panic!("字符索引超出字符范围"))
        .0;

    let to_insert = match &**args.get(1).unwrap() {
        Litr::Str(s) => s,
        _ => panic!("str.insert第二个参数必须是Str"),
    };

    s.insert_str(index, to_insert);
    Litr::Uninit
}

/// slice函数的内部函数
fn _slice(s: &mut str, args: Vec<CalcRef>) -> String {
    let mut indice = s.char_indices();
    let start = args.first().map_or(0, |n| to_usize(n));
    let end = args.get(1).map_or(s.len(), |n| to_usize(n));
    assert!(start <= end, "起始索引{}不可大于结束索引{}", start, end);

    let slice_start = indice
        .nth(start)
        .unwrap_or_else(|| panic!("起始索引{}超出字符范围", start))
        .0;
    let slice_end = indice.nth(end - start - 1).unwrap_or((s.len(), '\x00')).0;

    unsafe { s.get_unchecked(slice_start..slice_end).to_string() }
}

/// str转utf16 buf
fn to_utf16(s: &mut str) -> Litr {
    let v16: Vec<u16> = s.encode_utf16().collect();
    unsafe {
        Litr::Buf(std::slice::from_raw_parts(v16.as_ptr() as *const u8, v16.len() * 2).to_vec())
    }
}

/// 得到一个按行的迭代器
fn lines(s: &mut str) -> Litr {
    let v = Box::into_raw(Box::new(s.lines())) as usize;
    Litr::Ninst(NativeInstance {
        cls: unsafe { ITER_LINES },
        v,
        w: 0,
    })
}

/// 替换所有匹配字符 可传入第三个参数代表替换次数
fn _replace(s: &mut str, args: Vec<CalcRef>) -> String {
    assert!(args.len() >= 2, "str.replace需要传入匹配字符串和替换字符串");
    let from = match &**args.first().unwrap() {
        Litr::Str(s) => s,
        _ => panic!("str.replace第一个参数必须是Str"),
    };
    let to = match &**args.get(1).unwrap() {
        Litr::Str(s) => s,
        _ => panic!("str.replace第二个参数必须是Str"),
    };

    if let Some(times) = args.get(2) {
        let times = to_usize(times);
        s.replacen(from, to, times)
    } else {
        s.replace(from, to)
    }
}

/// 把字符串以一个分隔符分割成字符串列表
/// 第二个参数可以传true,让分割后的字符串保留分隔符
fn split(s: &mut str, args: Vec<CalcRef>) -> Litr {
    let with = args.first().map_or("", |s| match &**s {
        Litr::Str(s) => s,
        _ => panic!("str.split第一个参数必须是字符串"),
    });

    // 如果传true就用inclusive
    if let Some(n) = args.get(1) {
        if let Litr::Bool(b) = &**n {
            if *b {
                return Litr::List(
                    s.split_inclusive(with)
                        .map(|s| Litr::Str(s.to_string()))
                        .collect(),
                );
            }
        }
    }

    Litr::List(s.split(with).map(|s| Litr::Str(s.to_string())).collect())
}

/// 删除一段字符,并在删除处插入一段字符串
fn splice(s: &mut String, args: Vec<CalcRef>) -> Litr {
    let mut indice = s.char_indices();
    let start = args.first().map_or(0, |n| to_usize(n));
    let end = args.get(1).map_or(s.len(), |n| to_usize(n));
    assert!(start <= end, "起始索引{}不可大于结束索引{}", start, end);

    let slice_start = indice
        .nth(start)
        .unwrap_or_else(|| panic!("起始索引{}超出字符范围", start))
        .0;
    let slice_end = indice.nth(end - start - 1).unwrap_or((s.len(), '\x00')).0;

    let with = args.get(2).map_or("", |s| match &**s {
        Litr::Str(s) => s,
        _ => panic!("str.splice第三个参数必须是Str"),
    });

    s.replace_range(slice_start..slice_end, with);
    Litr::Uninit
}

// - statics -
pub fn statics() -> Vec<(Interned, NativeFn)> {
    use std::str::Lines;
    unsafe {
        // 初始化lines()迭代器类
        ITER_LINES = Box::into_raw(Box::new(super::new_iter_class(
            b"Str.lines",
            |v| {
                let itr = v.v as *mut Lines;
                (*itr)
                    .next()
                    .map_or(sym::iter_end(), |v| Litr::Str(v.to_string()))
            },
            |v| drop(Box::from_raw(v.v as *mut Lines)),
        )));
    }

    vec![
        (intern(b"from"), s_from),
        (intern(b"from_utf8"), s_from_utf8),
        (intern(b"from_utf16"), s_from_utf16),
    ]
}

/// 调用Litr::str
fn s_from(args: Vec<CalcRef>, _cx: Scope) -> Litr {
    let s = args.first().map_or(&Litr::Uninit, |s| &**s);
    Litr::Str(s.str())
}

/// utf8 buf to str 强检查版
fn s_from_utf8(args: Vec<CalcRef>, _cx: Scope) -> Litr {
    Litr::Str(args.first().map_or(String::new(), |s| match &**s {
        Litr::Buf(s) => String::from_utf8(s.clone()).expect("Str解析错误 非法utf8字符"),
        _ => panic!("Str::from_utf8第一个参数必须是Buf"),
    }))
}

/// utf16 buf to str 强检查版
fn s_from_utf16(args: Vec<CalcRef>, _cx: Scope) -> Litr {
    Litr::Str(args.first().map_or(String::new(), |s| {
        match &**s {
            Litr::Buf(s) => String::from_utf16(unsafe {
                std::slice::from_raw_parts(s.as_ptr() as *const u16, s.len() / 2)
            })
            .expect("Str解析错误 非法utf16字符"),
            _ => panic!("Str::from_utf16第一个参数必须是Buf"),
        }
    }))
}
