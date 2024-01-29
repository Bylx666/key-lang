
use crate::{
  ast::{
    Executable, ModDef
  }, 
  c::Clib,
  intern
};
use std::ptr::null_mut;
use std::ffi::CString;


#[repr(C)]
struct ExportedFuncs {
  len:usize,
  vec:*mut (*mut i8, Executable)
}

pub fn parse(name:&[u8],path:&[u8])-> Result<ModDef, String> {
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
      funcs = Vec::from_raw_parts(expfns.vec, expfns.len, expfns.len).into_iter().map(|(cstr, exec)|{
        let ident = intern(CString::from_raw(cstr).to_bytes());
        (ident, exec)
      }).collect();
    }
  }
  Ok(ModDef{
    name:intern(name),
    funcs
  })
}
