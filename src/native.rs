//! 提供Native Module的接口

use crate::{
  c::Clib, intern::{intern, Interned}, scan::{literal::{Function, Litr}, stmt::LocalMod}
};
use crate::runtime::{calc::CalcRef, Scope};

pub type NativeFn = fn(Vec<CalcRef>, Scope)-> Litr;
pub type NativeMethod = fn(*mut NativeInstance, args:Vec<CalcRef>, Scope)-> Litr;
pub type Getter = fn(*mut NativeInstance, get:Interned)-> Litr;
pub type Setter = fn(*mut NativeInstance, set:Interned, to:Litr);

#[derive(Debug, Clone)]
pub struct NativeMod {
  pub funcs: Vec<(Interned, NativeFn)>,
  pub classes: Vec<*mut NativeClassDef>
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct NativeClassDef {
  pub name: Interned,
  pub statics: Vec<(Interned, NativeFn)>,
  pub methods: Vec<(Interned, NativeMethod)>,
  pub getter: Getter,
  pub setter: Setter,
  pub index_get: fn(*mut NativeInstance, CalcRef)-> Litr,
  pub index_set: fn(*mut NativeInstance, CalcRef, Litr),
  pub next: fn(*mut NativeInstance)-> Litr,
  pub onclone: fn(*mut NativeInstance)-> NativeInstance,
  pub ondrop: fn(*mut NativeInstance)
}

/// 传进main里的东西，作为与原生的接口
#[repr(C)]
struct NativeInterface {
  intern: fn(&[u8])-> Interned,
  err: fn(&str)->!,
  find_var: fn(Scope, Interned)-> Option<CalcRef>,
  funcs: *mut Vec<(Interned, NativeFn)>,
  classes: *mut Vec<*mut NativeClassDef>
}

/// 原生类型实例
#[derive(Debug)]
#[repr(C)]
pub struct NativeInstance {
  pub v1: usize,
  pub v2: usize,
  pub cls: *mut NativeClassDef,
}

impl Clone for NativeInstance {
  /// 调用自定义clone (key-native库中的默认clone行为也可用)
  fn clone(&self) -> Self {
    (unsafe{&*self.cls}.onclone)(self as *const NativeInstance as *mut NativeInstance)
  }
}
impl Drop for NativeInstance {
  /// 调用自定义drop (key-native的默认drop不做任何事)
  fn drop(&mut self) {
    (unsafe{&*self.cls}.ondrop)(self)
  }
}

/// Litr中使用的NativeMethod类型
#[derive(Debug, Clone)]
pub struct BoundNativeMethod {
  pub bind: *mut NativeInstance,
  pub f: NativeMethod
}

fn err(s:&str)->! {
  panic!("{} \n  运行时({})", s, unsafe{crate::runtime::LINE})
}

pub fn parse(path:&[u8])-> Result<*const NativeMod, String> {
  let lib = Clib::load(path)?;
  let mut m = Box::new(NativeMod {
    funcs: Vec::new(), classes: Vec::new()
  });
  unsafe {
    let keymain:extern fn(&mut NativeInterface) = std::mem::transmute(lib.get(b"keymain").ok_or("模块需要'KeyMain'作为主运行函数")?);
    keymain(&mut NativeInterface {
      intern, err, find_var: Scope::var,
      funcs: &mut m.funcs, classes: &mut m.classes
    });
  }
  Ok(Box::into_raw(m))
}
