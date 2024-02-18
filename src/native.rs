//! 提供Native Module的接口

use crate::{
  c::Clib, intern::{intern, Interned}, scan::{literal::{Function, Litr}, stmt::LocalMod}
};

pub type NativeFn = fn(Vec<Litr>)-> Litr;
pub type NativeMethod = fn(*mut NativeInstance, Vec<Litr>)-> Litr;
pub type Getter = fn(get:Interned)-> Litr;
pub type Setter = fn(set:Interned, to:Litr);
pub type IndexGetter = fn(get:usize)-> Litr;
pub type IndexSetter = fn(set:usize, to:Litr);

#[derive(Debug, Clone)]
pub struct NativeMod {
  pub name: Interned,
  pub funcs: Vec<(Interned, NativeFn)>,
  pub classes: Vec<*const NativeClassDef>
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct NativeClassDef {
  pub name: Interned,
  pub getter: Getter,
  pub setter: Setter,
  pub igetter: IndexGetter,
  pub isetter: IndexSetter,
  pub onclone: NativeMethod,
  pub ondrop: NativeMethod,
  pub statics: Vec<(Interned, NativeFn)>,
  pub methods: Vec<(Interned, NativeMethod)>
}

/// 传进main里的东西，作为与原生的接口
#[repr(C)]
struct NativeInterface {
  intern: fn(&[u8])-> Interned,
  err: fn(&str)->!,
  funcs: *mut Vec<(Interned, NativeFn)>,
  classes: *mut Vec<*const NativeClassDef>
}

/// 原生类型实例
#[derive(Debug, Clone)]
#[repr(C)]
pub struct NativeInstance {
  pub v1: usize,
  pub v2: usize,
  pub cls: *mut NativeClassDef,
}

/// Litr中使用的NativeMethod类型
#[derive(Debug, Clone)]
pub struct BoundNativeMethod {
  pub bind: *mut NativeInstance,
  pub f: NativeMethod
}

pub fn parse(name:Interned,path:&[u8])-> Result<NativeMod, String> {
  let lib = Clib::load(path)?;
  let mut funcs = Vec::new();
  let mut classes = Vec::new();
  fn err(s:&str)->! {
    panic!("{} \n  运行时({})", s, unsafe{crate::runtime::LINE})
  }
  unsafe {
    let keymain:extern fn(&mut NativeInterface) = std::mem::transmute(lib.get(b"keymain").ok_or("模块需要'KeyMain'作为主运行函数")?);
    keymain(&mut NativeInterface {
      intern, err, funcs: &mut funcs, classes: &mut classes
    });
  }
  Ok(NativeMod{
    name,
    funcs,
    classes
  })
}
