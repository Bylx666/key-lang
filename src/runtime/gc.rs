
use std::{ops::{Deref, DerefMut}, ptr::NonNull};
use crate::ast::LocalFuncInner;
use super::ScopeInner;
use std::sync::atomic::Ordering;


/// 针对垃圾回收实现的本地函数指针
/// 
/// 需要在其drop前手动增加和减少其对应作用域的引用计数
#[derive(Debug)]
pub struct LocalFunc {
  /// pointer，
  /// 不要pub这玩意，外部用起来会感觉很乱
  p:NonNull<LocalFuncInner>
}
impl LocalFunc {
  /// 将绑定过作用域的本地函数包装到指针里
  pub fn new(f:LocalFuncInner)-> Self {
    let p = LocalFunc{
      p: NonNull::new(Box::into_raw(Box::new(f))).unwrap()
    };
    p.count_enc();
    p
  }
  /// 增加一层引用计数
  pub fn count_enc(&self) {
    unsafe{
      let f = self.p.as_ref();
      let mut scope = f.scope;
      loop {
        scope.count.fetch_add(1, Ordering::Relaxed);
        if let Some(prt) = scope.parent {
          scope = prt;
        }else {
          break;
        }
      }
    }
  }
  /// 减少一层引用计数
  pub fn count_dec(&self) {
    unsafe{
      let f = self.p.as_ref();
      let mut scope = f.scope;
      loop {
        scope.count.fetch_sub(1, Ordering::Relaxed);
        if let Some(prt) = scope.parent {
          scope = prt;
        }else {
          break;
        }
      }
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

impl Clone for LocalFunc {
  fn clone(&self) -> Self {
    self.count_enc();
    LocalFunc {
      p: self.p.clone()
    }
  }
}

// impl Drop for LocalFunc {
//   fn drop(&mut self) {
//     // 减少一层引用计数
//     self.count_dec();
//     use std::ptr::drop_in_place;
//     unsafe{
//       let mut scope = self.scope;
//       // 当前作用域无引用时直接回收函数定义的内存
//       if scope.count.load(Ordering::Relaxed) == 0 {
//         drop_in_place(self.p.as_ptr());
//       }
//       loop {
//         if scope.count.load(Ordering::Relaxed) == 0 {
//           drop_in_place(scope.p.as_ptr());
//           if let Some(prt) = scope.parent {
//             scope = prt;
//           }else {
//             break;
//           }
//         }else {
//           // 子作用域计数非零，则父作用域计数必定非0
//           break;
//         }
//       }
//     }
//   }
// }

