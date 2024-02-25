//! 垃圾回收使用Outlive算法实现
//! 
//! 具体思路见Outlives结构

use super::{LocalFuncRaw, LocalFunc};
use std::sync::atomic::Ordering;


use std::sync::atomic::AtomicUsize;
use super::Scope;

/// 作用域的outlive引用计数系统
#[derive(Debug, Default)]
pub struct Outlives {
  /// 该作用域定义函数生命周期被延长(被outlive)的次数
  /// 
  /// 若作用域结束时该值大于0就会等被延长函数生命周期结束后再回收
  count: AtomicUsize,
  /// 该作用域得到的延长了生命周期的函数列表
  /// 
  /// 在该作用域结束时会为列表中所有函数减少一层`count`
  to_drop: Vec<LocalFunc>
}
impl Outlives {
  pub fn new()-> Self {
    Outlives {
      count:AtomicUsize::new(0),
      to_drop:Vec::new()
    }
  }
}

/// 将函数生命周期延长至目标作用域结束
/// 
/// 目标作用域如果在函数定义处的作用域内就无效果(因为生命周期根本不会变长)
/// 
/// 为此函数定义处的作用域和其所有父作用域增加一次引用计数
/// 
/// 并为outlive的目标作用域托付管理一层该函数的引用计数
pub fn outlive_to(f:LocalFunc, mut to:Scope) {
  if to.subscope_of(f.scope) {
    return;
  };
  outlive_static(f.scope);
  to.outlives.to_drop.push(f);
}

/// 将作用域生命周期延长至永久
/// 
/// 只会增加一层作用域的引用计数，但不会托付to_drop
/// 
/// 程序期间不会回收此作用域，目前只用于ks模块导出函数
pub fn outlive_static(mut scope:Scope) {
  loop {
    scope.outlives.count.fetch_add(1, Ordering::Relaxed);
    if let Some(prt) = scope.parent {
      scope = prt;
    }else {
      break;
    }
  }
}


/// 作用域结束时使用此函数来回收作用域中引用到的所有函数
/// 
/// 若引用计数为0就回收作用域
pub fn scope_end(mut scope:Scope) {
  // 回收作用域本身
  if scope.outlives.count.load(Ordering::Relaxed) == 0 {
    unsafe { std::ptr::drop_in_place(scope.ptr) }
  }

  /// 作用域减少一层引用计数
  #[inline]
  fn sub_count(mut scope: Scope) {
    loop {
      let prev = scope.outlives.count.fetch_sub(1, Ordering::Relaxed);
      if prev == 1 && scope.ended {
        unsafe{ std::ptr::drop_in_place(scope.ptr) }
      }
      if let Some(prt) = scope.parent {
        scope = prt;
      }else {
        break;
      }
    }
  }

  let mut to_drop = std::mem::take(&mut scope.outlives.to_drop);
  for f in to_drop.into_iter() {
    let scope = f.scope;
    sub_count(scope);
  }
}


/// 如果值包含本地函数就为函数定义处增加一层引用计数
pub fn may_add_ref(v:&crate::scan::literal::Litr, target_scope: Scope) {
  use crate::scan::literal::{Litr, Function};
  match v {
    Litr::Func(f)=> {
      if let Function::Local(f) = f {
        outlive_to(f.clone(),target_scope);
      }
    }
    Litr::List(l)=> 
      l.iter().for_each(|item|may_add_ref(item, target_scope)),
    Litr::Inst(inst)=> 
      inst.v.iter().for_each(|item|may_add_ref(item, target_scope)),
    Litr::Obj(map)=> map.values().for_each(|item|may_add_ref(item, target_scope)),
    _=> ()
  }
}
