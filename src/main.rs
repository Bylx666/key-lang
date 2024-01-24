#![allow(unused)]
#![feature(hash_set_entry)]

use std::{fs, collections::HashMap, mem::transmute, hash::{BuildHasher, Hash}, vec};
use std::process::ExitCode;

mod intern;
use intern::intern;
mod ast;
mod scan;
mod runtime;
mod allocated;
mod utils;
mod extern_agent;

fn main()-> ExitCode {
  // 自定义panic
  // std::panic::set_hook(Box::new(|inf| {
  //   let str = inf.payload().downcast_ref::<String>();
  //   if let Some(s) = str {
  //     println!("\n> {}\n\n> Key Script CopyLeft by Subkey\n  {}\n", s, utils::date());
  //   }else {
  //     println!("\n{}\n\nKey Script CopyLeft by Subkey\n{}\n", inf, utils::date());
  //   }
  // }));

  // 现在内存泄漏完了
  // is
  // ?
  // evil

  intern::init();
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  let exit = runtime::run(&scanned);
  if let ast::Litr::Int(code) = exit {
    return ExitCode::from(code as u8);
  }
  ExitCode::SUCCESS

  // 别忘了做注释解析
}