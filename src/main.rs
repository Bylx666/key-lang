#![allow(unused)]
#![feature(hash_set_entry)]

use std::{fs, collections::HashMap, mem::transmute, hash::{BuildHasher, Hash}, vec};
use std::process::ExitCode;

mod intern;
mod scan;
mod runtime;
mod primitive;
mod utils;

mod c;
mod native;

/// 标志目前走到的行号
static mut LINE:usize = 0;
/// 用于标记报错文件
static mut PLACE:String = String::new();

/// 标志解释器的版本
static VERSION:usize = 100000;

/// 解释器发行者(用于区分主版本和魔改版)
/// 
/// 如果需要自己魔改,且需要考虑和主版本的兼容性可以更改此值
/// 
/// 用户可以使用distribution()直接读取此值
static DISTRIBUTION:&str = "Subkey";

fn main()-> ExitCode {
  // 自定义报错
  // std::panic::set_hook(Box::new(|inf| {
  //   use crate::utils::date;
  //   let line = unsafe{LINE};
  //   let s = if let Some(mes) = inf.payload().downcast_ref::<&'static str>() {
  //     mes
  //   }else if let Some(mes) = inf.payload().downcast_ref::<String>() {
  //     mes
  //   }else{"错误"};
  //   println!("\n> {}\n  第{}行\n\n> Key Script CopyLeft by Subkey\n  {}\n", s, line, date());
  // }));

  // 基本类型的方法，也就是所有litr的prop
  // 字符串捕获变量
  // str的index
  // let [] = x
  // let a=0,b=0
  // prelude mod 让模块本身帮你初始化上下文
  // 20 |> f1(2,|%|,4) |> 

  // pub use
  // ERR_PLACE报错显示文件, 导入文件的路径检测
  // newInst如果属性不全不让构造
  // 传进Native的struct怎么处理？
  // for i {func(){i}}内部的i是否正确
  // Native outlive api, @display,@to_str
  // key intern
  // 可变参数([args])
  // let * some = 20 -> *some == 20 常量const
  // ..[参数展开]
  // extern to_raw_args
  // throw catch
  // 参数类型检查
  // 同名省略struct属性
  // 如果不加分号报错会错行，记得提示用户
  // 科学计数法
  // wasm
  // 脚本打包exe

  intern::init();
  let scanned = scan::scan(&fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  let exit = runtime::run(&scanned);
  if let primitive::litr::Litr::Int(code) = exit.returned {
    return ExitCode::from(code as u8);
  }
  ExitCode::SUCCESS

}