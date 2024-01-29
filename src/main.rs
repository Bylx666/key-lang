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

mod c;
mod extern_agent;
mod module;

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

  // let & some = 20 显式指定指针变量
  // cstruct
  // 模块：用户模块user mod和底层模块native mod
  // 设计针对作用域的gc
  // Str之类的表达式会被clone，指针估计对不上了吧(runtime::calc)
  // mem::swap
  // extern to_raw_args
  // 别忘了做注释解析
  // is
  // ?var
  // evil
  // 同名省略struct属性
  // 如果不加分号报错会错行，记得提示用户

  intern::init();
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  // println!("{scanned:?}");
  let exit = runtime::run(&scanned);
  if let ast::Litr::Int(code) = exit {
    return ExitCode::from(code as u8);
  }
  ExitCode::SUCCESS

}