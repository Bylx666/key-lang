#![allow(unused)]
use std::{fs, collections::HashMap};
mod ast;
mod scan;
mod runtime;

fn date()-> String {
  #[inline]
  fn is_leap(year: u64)-> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 4 == 400
  }
  let t = std::time::SystemTime::now();
  let mut t = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
  let sec = t % 60;
  t /= 60;
  let min = t % 60;
  t /= 60;
  let hour = t % 24 + 8;
  t /= 24;
  let mut year = 1970;
  while t >= 365 {
    t -= 365;
    if is_leap(year) {
      if t>=1 {t -= 1}
      else {break;}
    }
    year += 1;
  }

  let mut month_list:[u64;12] = [31,28,31,30,31,30,31,31,30,31,30,31];
  if is_leap(year) {month_list[1] = 29};

  let mut mon:u8 = 1;
  for n in month_list {
    if t >= n {t -= n;}
    else {break;}
    mon += 1;
  }
  format!("{year}/{mon:02}/{t:02} {hour:02}:{min:02}:{sec:02}")
}

fn main() {
  // 自定义panic
  // std::panic::set_hook(Box::new(|inf| {
  //   let str = inf.payload().downcast_ref::<String>();
  //   if let Some(s) = str {
  //     println!("\n> {}\n\n> Key Script CopyLeft by Subkey\n  {}\n", s, date());
  //   }else {
  //     println!("\n{}\n\nKey Script CopyLeft by Subkey\n{}\n", inf, date());
  //   }
  // }));

  // 目前只能单参数，args类型的expr仍未完成。
  // 运算符优先级：括号的使用
  // 分号省略可以考虑一下怎么实现
  // 比如 a()b+2是两个语句

  let mut scope = runtime::top_scope();
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  scope.run(&scanned);

  println!("{:?}", scanned.exec);
  // 解析过程可以把[i]优化成get_unchecked
  // 要把Ident优化成usize
  // 别忘了做注释解析
  // 字符串缓存池
}