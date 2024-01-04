#![allow(unused)]
use std::{fs, collections::HashMap, mem::transmute};
mod ast;
mod scan;
mod runtime;
mod utils;

fn main() {
  // 自定义panic
  std::panic::set_hook(Box::new(|inf| {
    let str = inf.payload().downcast_ref::<String>();
    if let Some(s) = str {
      println!("\n> {}\n\n> Key Script CopyLeft by Subkey\n  {}\n", s, utils::date());
    }else {
      println!("\n{}\n\nKey Script CopyLeft by Subkey\n{}\n", inf, utils::date());
    }
  }));

  let mut scope = runtime::top_scope();
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  // println!("{:?}", scanned);
  scope.run(&scanned);

  // 别忘了做注释解析
  // 字符串缓存池
}