//! 定义顶级作用域的函数

use crate::intern::{Interned,intern};
use crate::primitive::litr::{Litr, Function};
use crate::runtime::{calc::CalcRef, Scope, Variant};

pub fn prelude()-> Vec<Variant> {
  macro_rules! prel {($($name:literal:$f:ident)*)=>{
    vec![$( Variant {
      name: intern($name),
      locked: true,
      v: Litr::Func(Function::Native($f))
    }, )*]
  }}
  prel!{
    b"log":log
    b"run_ks":run_ks
    b"version":version
    b"distribution":distribution
    b"swap":swap
    b"take":take
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
pub fn run_ks(args:Vec<CalcRef>, mut cx:Scope)-> Litr {
  // 设置报错位置到evil
  use crate::{PLACE, LINE};
  unsafe{PLACE = format!("{}({}) when eviling", PLACE, LINE);}

  let s = args.get(0).expect("evil需要传入一个被解析的字符串或数组");
  let s = match &**s {
    Litr::Str(s)=> s.as_bytes(),
    Litr::Buf(b)=> &**b,
    _=> panic!("evil只能运行字符串或数组")
  };
  let scanned = crate::scan::scan(s);

  // 运行
  for (l, sm) in &scanned.0 {
    unsafe{
      LINE = *l;
    }
    cx.evil(sm);
    // 如果evil到return或break就在这停下
    if cx.ended {
      break;
    }
  }
  Litr::Uninit
}

/// 获取版本号
fn version(_a:Vec<CalcRef>, _c:Scope)-> Litr {
  Litr::Uint(crate::VERSION)
}

/// 获取发行者
fn distribution(_a:Vec<CalcRef>, _c:Scope)-> Litr {
  Litr::Str(crate::DISTRIBUTION.to_string())
}

/// 无分配的直接交互数值
fn swap(mut args:Vec<CalcRef>, _c:Scope)-> Litr {
  assert!(args.len()>=2, "swap需要两个值用于无分配交换");
  let mut it = args.iter_mut();
  std::mem::swap(&mut **it.next().unwrap(), &mut **it.next().unwrap());
  Litr::Uninit
}

/// 无分配的直接取走一个值, 并将原值变为uninit
fn take(mut args:Vec<CalcRef>, _c:Scope)-> Litr {
  let a = args.get_mut(0).expect("take需要一个被取走的值");
  let mut b = Litr::Uninit;
  std::mem::swap(&mut **a, &mut b);
  b
}
