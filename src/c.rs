//! 所有跨平台相关的函数都在这

extern {
  fn LoadLibraryA(src:*const u8)-> usize;
  fn GetProcAddress(lib:usize, src:*const u8)-> usize;
}

pub unsafe fn dlopen(src:*const u8)-> usize {
  unsafe {LoadLibraryA(src)}
}

pub unsafe fn dlsym(lib:usize, src:*const u8)-> usize {
  unsafe {GetProcAddress(lib, src)}
}


pub struct Clib (usize);
impl Clib {
  /// 加载一个动态库
  pub fn load(s:&[u8])-> Result<Self, String> {
    unsafe {
      let lib = dlopen([s,&[0]].concat().as_ptr());
      if lib == 0 {
        Err(format!("无法找到动态库'{}'",String::from_utf8_lossy(s)))
      }else {
        Ok(Clib(lib))
      }
    }
  }
  /// 从动态库中寻找一个函数
  pub fn get(&self, sym:&[u8])-> Option<usize> {
    let mut s = [sym,&[0]].concat();
    unsafe {
      let v = dlsym(self.0, s.as_ptr());
      if v == 0 {
        return None;
      }
      Some(v)
    }
  }
}