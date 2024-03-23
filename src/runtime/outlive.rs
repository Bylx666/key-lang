//! 垃圾回收使用Outlive算法实现
//! 
//! 具体思路见Outlives结构

use super::LocalFuncRaw;
use std::sync::atomic::Ordering;


use std::sync::atomic::AtomicUsize;
use super::Scope;
fn ln()->usize{unsafe{crate::LINE}}

/// 本地函数指针
#[derive(Debug)]
pub struct LocalFunc {
  /// pointer
  pub ptr:*const LocalFuncRaw,
  /// 来自的作用域
  pub scope: Scope,
}

impl LocalFunc {
  /// 将本地函数定义和作用域绑定
  pub fn new(ptr:*const LocalFuncRaw, scope: Scope)-> Self {
    // 创建时加一层, 对应作用域结束时的减一层
    // println!("{:02}: func new : {:p}",ln(),ptr);
    increase_scope_count(scope);
    LocalFunc{
      ptr,
      scope
    }
  }
}

impl std::ops::Deref for LocalFunc {
  type Target = LocalFuncRaw;
  fn deref(&self) -> &Self::Target {
    unsafe {&*self.ptr}
  }
}

impl Clone for LocalFunc {
  fn clone(&self) -> Self {
    let scope = self.scope;
    // println!("{:02}: func clone : {:p}",ln(), self.ptr);
    // 只要复制就加一次函数定义处作用域引用计数
    increase_scope_count(scope);
    LocalFunc {ptr: self.ptr, scope}
  }
}

// 因为Drop只有ks作用域释放才会触发, 所以必须把Drop手动实现在ks作用域结束的地方
// 但不是所有的LocalFunc都会被作用域持有
// 像是a=||{};0+a的写法会复制一遍a函数但被复制的部分无处持有,会被直接rust drop
// 或者a=||{};a=0的时候,此函数会直接原地drop
impl Drop for LocalFunc {
  fn drop(&mut self) {
    let count = &self.scope.outlives;
    if !self.scope.ended {
      // println!("{:02}: func drop inplace : {:?}",ln(), self.ptr);
      count.fetch_sub(1, Ordering::Relaxed);
    }
  }
}


/// 增加一层作用域的引用计数
pub fn increase_scope_count(mut scope:Scope) {
  loop {
    scope.outlives.fetch_add(1, Ordering::Relaxed);
    if let Some(prt) = scope.parent {
      scope = prt;
    }else {
      break;
    }
  }
}

/// 作用域减少一层引用计数
/// 需要保证scope.outlive大于0
pub fn decrease_scope_count(mut scope: Scope) {
  loop {
    let prev = scope.outlives.fetch_sub(1, Ordering::Relaxed);
    if prev == 1 && scope.ended {
      // println!("{:02}: scope drop by func: {:p}",ln(), scope.ptr);
      unsafe{ std::ptr::drop_in_place(scope.ptr) }
    }
    if let Some(prt) = scope.parent {
      scope = prt;
    }else {
      break;
    }
  }
}


/// 作用域结束时使用此函数来回收作用域中持有的所有函数
/// 
/// 若引用计数为0就回收作用域
pub fn scope_end(mut scope:Scope) {
  scope.ended = true;
  // 回收作用域本身
  if scope.outlives.load(Ordering::Relaxed) == 0 {
    // println!("{:02}: scope drop by end: {:p}",ln(), scope.ptr);
    unsafe { std::ptr::drop_in_place(scope.ptr) }
  }
  
  for (_, v) in &scope.vars {
    drop_func(v);
  }
}


/// 为一个Litr中所有LocalFunc减一层引用计数
pub fn drop_func(v:&crate::scan::literal::Litr) {
  use crate::scan::literal::{Litr, Function};
  match v {
    Litr::Func(f)=> {
      if let Function::Local(f) = f {
        // println!("{:02}: func drop by end : {:p}",ln(), f.ptr);
        decrease_scope_count(f.scope);
      }
    }
    Litr::List(l)=> l.iter().for_each(|item|drop_func(item)),
    Litr::Inst(inst)=> inst.v.iter().for_each(|item|drop_func(item)),
    Litr::Obj(map)=> map.values().for_each(|item|drop_func(item)),
    _=> ()
  }
}
