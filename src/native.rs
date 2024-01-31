//! 提供Native Module的接口

use crate::{
  ast::{
    Executable, ModDef
  }, 
  c::Clib,
  intern::{intern, Interned}
};
use std::ptr::null_mut;
use std::ffi::CStr;


struct ExportedFuncs {
  len:usize,
  vec:*mut (*mut i8, Executable)
}

pub fn parse(name:Interned,path:&[u8])-> Result<ModDef, String> {
  let lib = Clib::load(path)?;
  let mut funcs = Vec::new();
  unsafe {
    let keymainp = lib.get(b"KeyMain").ok_or("模块需要'KeyMain'作为主运行函数")?;
    let keymain:extern fn() = std::mem::transmute(keymainp);
    keymain();
    if let Some(f) = lib.get(b"GetExportedFuncs") {
      let mut expfns = ExportedFuncs {
        len: 0,
        vec: null_mut()
      };
      let f:extern fn(*mut ExportedFuncs) = std::mem::transmute(f);
      f(&mut expfns);
      funcs = std::slice::from_raw_parts(expfns.vec, expfns.len).iter().cloned().map(|(cstr, exec)|{
        let ident = intern(CStr::from_ptr(cstr).to_bytes());
        (ident, exec)
      }).collect();
    }
  }
  Ok(ModDef{
    name,
    funcs
  })
}
