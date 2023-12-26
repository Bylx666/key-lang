#![allow(unused)]
use std::{fs, collections::HashMap};
mod ast;
mod scan;

type Ident = Vec<u8>;

fn main() {
  let scanned = scan::scan(fs::read("D:\\code\\rs\\key-lang\\samples\\helloworld.ks").unwrap());
  println!("{:?}", scanned);
  // 要把Ident优化成usize
}