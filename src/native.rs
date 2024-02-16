//! 提供Native Module的接口

use crate::{
  scan::literal::{Litr, Function}, 
  scan::stmt::LocalModDef, 
  c::Clib,
  intern::{intern, Interned}
};

pub type NativeFn = fn(Vec<Litr>)-> Litr;

pub fn parse(name:Interned,path:&[u8])-> Result<LocalModDef, String> {
  let lib = Clib::load(path)?;
  let mut funcs = Vec::new();
  let mut classes = Vec::new();
  unsafe {
    let keymain:extern fn() = std::mem::transmute(lib.get(b"keymain").ok_or("模块需要'KeyMain'作为主运行函数")?);
    keymain();
    if let Some(f) = lib.get(b"GetExportedFuncs") {
      let mut expfns = Vec::new();
      let f:extern fn(*mut Vec::<(Box<[u8]>, NativeFn)>) = std::mem::transmute(f);
      f(&mut expfns);
      // funcs = expfns.into_iter().map(|(id, exec)|{
      //   let ident = intern(&id);
      //   (ident, Function::Native(exec))
      // }).collect();
    }
  }
  Ok(LocalModDef{
    name,
    funcs,
    classes
  })
}
