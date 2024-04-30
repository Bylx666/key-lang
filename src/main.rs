#![feature(hash_set_entry)]

use std::fs;
use std::process::ExitCode;

mod intern;
mod scan;
mod runtime;
mod primitive;
mod utils;

mod c;
mod native;

/// 全局选项
struct GlobalOptions {
  /// --ast
  print_ast: bool
}
static mut GLOBAL_OPTIONS:GlobalOptions = GlobalOptions {
  print_ast: false
};

/// 标志目前走到的行号
static mut LINE:usize = 1;
/// 用于标记目前文件路径 在模块导入搜索时使用此作为搜索目录
static mut FILE_PATH:&str = "";

/// 标志解释器的版本
static VERSION:usize = 100062;

/// 解释器发行者(用于区分主版本和魔改版)
/// 
/// 如果需要自己魔改,且需要考虑和主版本的兼容性可以更改此值
/// 
/// 用户可以使用distribution()直接读取此值
static DISTRIBUTION:&str = "Subkey";

fn main()-> ExitCode {
  // linux macos支持
  // 脚本打包exe

  intern::init();

  // 获取路径
  let mut args = std::env::args();
  args.next();
  let path = if let Some(s) = args.next() {
    utils::to_absolute_path(s).leak()
  }else {
    println!("> Key Lang\n  version: {}\n  by: {}", VERSION, DISTRIBUTION);
    return ExitCode::SUCCESS;
  };
  
  while let Some(n) = args.next() {
    let opts = unsafe {&mut GLOBAL_OPTIONS};
    match &*n {
      "--ast"=> opts.print_ast = true,
      _=>()
    }
  }

  // 自定义报错
  unsafe {FILE_PATH = path}
  std::panic::set_hook(Box::new(|inf| {
    use crate::utils::date;
    let line = unsafe{LINE};
    let place = unsafe{&*FILE_PATH};
    let s = if let Some(mes) = inf.payload().downcast_ref::<&'static str>() {
      mes
    }else if let Some(mes) = inf.payload().downcast_ref::<String>() {
      mes
    }else{"错误"};

    let stack = unsafe{
      let mut s = String::new();
      use std::fmt::Write;
      for n in runtime::call::CALL_STACK.iter().rev() {
        let _ = s.write_fmt(format_args!("\n    {} at {}:{}",n.fname,n.file,n.line));
      }
      s
    };
    println!("\n> {}\n  {}:第{}行{}\n\n> Key Script CopyLeft by {}\n  {}", s, place, line, stack, DISTRIBUTION, date());
  }));

  // 运行并返回
  let scanned = scan::scan(&fs::read(&path).unwrap_or_else(|e|
    panic!("无法读取'{}': {}", path, e)));
  if unsafe{GLOBAL_OPTIONS.print_ast} {println!("{scanned:?}")}

  let exit = runtime::run(&scanned, path);

  // 如果原生模块调用了wait_inc就堵住当前线程
  unsafe {
    let mut n = native::WAITING.lock().unwrap();
    while *n > 0 {
      n = native::WAITING_CVAR.wait(n).unwrap();
    }
  }

  if let primitive::litr::Litr::Int(code) = exit.returned {
    return ExitCode::from(code as u8);
  }
  ExitCode::SUCCESS

}