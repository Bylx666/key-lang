#![allow(unused)]
use std::{fs, collections::HashMap};
mod ast;
mod scan;
mod runtime;


fn main() {
  let scope = runtime::top_scope();
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  scope.run(&scanned);
  // println!("{:?}", runtime::top_scope());
  // 要把Ident优化成usize
}