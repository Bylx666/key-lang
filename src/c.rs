//! 所有跨平台相关的函数都在这

use std::ptr::NonNull;

extern {
  fn LoadLibraryA(src:*const u8)-> *const ();
  fn GetProcAddress(lib:*const (), src:*const u8)-> *const ();
}

pub unsafe fn dlopen(src:*const u8)-> *const () {
  unsafe {LoadLibraryA(src)}
}

pub unsafe fn dlsym(lib:*const (), src:*const u8)-> *const () {
  unsafe {GetProcAddress(lib, src)}
}


pub struct Clib (*const ());
impl Clib {
  /// 加载一个动态库
  pub fn load(s:&[u8])-> Self {
    unsafe {
      let lib = dlopen([s,&[0]].concat().as_ptr());
      if lib.is_null() {
        panic!("无法找到动态库'{}'",String::from_utf8_lossy(s))
      }else {
        Clib(lib)
      }
    }
  }
  /// 从动态库中寻找一个函数
  pub fn get(&self, sym:&[u8])-> Option<*const ()> {
    let mut s = [sym,&[0]].concat();
    unsafe {
      let v = dlsym(self.0, s.as_ptr());
      if v.is_null() {
        None
      }else {
        Some(v)
      }
    }
  }
}