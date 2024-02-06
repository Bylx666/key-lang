//! Gc的实现依赖于本地函数对作用域的引用计数
//! 
//! 因此为本地函数实现引用计数控制就是Gc行为本身

use std::{ops::{Deref, DerefMut}, ptr::NonNull};
use crate::ast::LocalFuncInner;
use super::ScopeInner;
use std::sync::atomic::Ordering;


/// 针对垃圾回收实现的本地函数指针
/// 
/// 需要手动增加和减少其对应作用域的引用计数
/// 
/// 作用域后于函数释放，因此你不可能依靠其自动析构来帮你检测和释放
#[derive(Debug, Clone)]
pub struct LocalFunc {
  /// pointer，
  /// 不要pub这玩意，外部用起来会感觉很乱
  p:NonNull<LocalFuncInner>
}
impl LocalFunc {
  /// 将绑定过作用域的本地函数包装到指针里
  /// 
  /// 此行为会自动增加一层引用计数
  pub fn new(f:LocalFuncInner)-> Self {
    let p = LocalFunc{
      p: NonNull::new(Box::into_raw(Box::new(f))).unwrap()
    };
    p.count_enc();
    p
  }
  /// 增加一层引用计数
  /// 
  /// 只有在定义函数和复制(赋值，传参，传列表之类的)行为时会调用一次
  pub fn count_enc(&self) {
    unsafe{
      let f = self.p.as_ref();
      let mut scope = f.scope;
      println!("enc:");
      loop {
        scope.count.fetch_add(1, Ordering::Relaxed);
        println!("{}",scope.count.load(Ordering::Relaxed));
        if let Some(prt) = scope.parent {
          scope = prt;
        }else {
          break;
        }
      }
    }
  }
  /// 减少一层引用计数
  /// 
  /// 在作用域结束时会为其所有定义的函数调用一次
  pub fn count_dec(&self) {
    unsafe{
      let f = self.p.as_ref();
      let mut scope = f.scope;
      println!("dec:");
      loop {
        scope.count.fetch_sub(1, Ordering::Relaxed);
        println!("{}",scope.count.load(Ordering::Relaxed));
        if let Some(prt) = scope.parent {
          scope = prt;
        }else {
          break;
        }
      }
    }
  }
  /// 作用域后于函数释放，因此你不可能依靠其自动析构来帮你检测和释放，
  /// 只能手动啦
  pub fn drop(&mut self) {
    unsafe{
      std::ptr::drop_in_place(self.p.as_ptr());
    }
  }
}

impl Deref for LocalFunc {
  type Target = LocalFuncInner;
  fn deref(&self) -> &Self::Target {
    unsafe {self.p.as_ref()}
  }
}
impl DerefMut for LocalFunc {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {self.p.as_mut()}
  }
}
