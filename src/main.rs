#![allow(unused)]
#![feature(hash_set_entry)]

use std::{fs, collections::HashMap, mem::transmute, hash::{BuildHasher, Hash}, vec};
use std::process::ExitCode;

mod intern;
use intern::intern;
mod ast;
mod scan;
mod runtime;
mod utils;

mod c;
mod native;

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

  // native cls
  // 基本类型的方法
  // 用cell代替ref
  // 析构函数..?
  // 指针self链式调用

  // 传进Native的struct怎么处理？
  // Obj的ord
  // for i {func(){i}}内部的i是否正确
  // func bind
  // Native outlive api
  // 本地函数字面量的call行为 ||{}() ||:20;()
  // ^ 或许还能开一个强制分号模式
  // 连等似乎可以直接runtime里if let
  // !{}
  // 基本的语句好像还没实现完呢
  // 可变参数([args])
  // let & some = 20 -> usize 显式指定指针变量
  // let * some = 20 -> *some == 20 常量const
  // key关键词到底有什么用啊
  // ..[参数展开]
  // buffer的from_raw实现记得区分rust和clone版
  // 模块：用户模块user mod和底层模块native mod
  // mem::swap
  // extern to_raw_args
  // is
  // evil
  // throw catch
  // 参数类型检查
  // Instance::set_any()
  // 同名省略struct属性
  // 如果不加分号报错会错行，记得提示用户

  intern::init();
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  println!("{scanned:?}");
  let exit = runtime::run(&scanned);
  if let ast::Litr::Int(code) = exit.returned {
    return ExitCode::from(code as u8);
  }
  ExitCode::SUCCESS

}