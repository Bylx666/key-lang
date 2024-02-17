//! 提供Native Module的接口

use crate::{
  c::Clib, intern::{intern, Interned}, scan::{literal::{Function, Litr}, stmt::LocalMod}
};

pub type NativeFn = fn(Vec<Litr>)-> Litr;
pub type Getter = fn(get:&[u8]);
pub type Setter = fn(set:&[u8], to:Litr);
pub type IndexGetter = fn(get:usize)-> Litr;
pub type IndexSetter = fn(set:usize, to:Litr)-> Litr;
pub type NativeMethod = fn(kself:&mut Litr, Vec<Litr>)-> Litr;

#[derive(Debug, Clone)]
pub struct NativeMod {
  pub name: Interned,
  pub funcs: Vec<(Interned, NativeFn)>,
  pub classes: Vec<NativeClassDef>
}

#[derive(Debug, Clone)]
pub struct NativeClassDef {
  pub name: Interned,
  pub getters: Vec<(Interned, Getter)>,
  pub setters: Vec<(Interned, Setter)>,
  pub igetters: Vec<(Interned, IndexGetter)>,
  pub isetters: Vec<(Interned, IndexSetter)>,
  pub statics: Vec<(Interned, NativeFn)>,
  pub methods: Vec<(Interned, NativeMethod)>
}

pub struct NativeApis<'a> {
  export_fn: &'a mut dyn FnMut(&'a [u8], NativeFn),
  export_cls: &'a mut dyn FnMut(NativeClassDef),
}

#[repr(transparent)]
pub struct NativeInstance {
  pub p: usize
}

pub fn parse(name:Interned,path:&[u8])-> Result<NativeMod, String> {
  let lib = Clib::load(path)?;
  let mut funcs = Vec::new();
  let export_fn = &mut |name, f|funcs.push((intern(name), f));
  let mut classes = Vec::new();
  let export_cls = &mut |c|classes.push(c);
  unsafe {
    let keymain:extern fn(&mut NativeApis) = std::mem::transmute(lib.get(b"keymain").ok_or("模块需要'KeyMain'作为主运行函数")?);
    keymain(&mut NativeApis {
      export_fn, export_cls
    });
  }
  Ok(NativeMod{
    name,
    funcs,
    classes
  })
}
