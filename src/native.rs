//! 提供Native Module的接口

use crate::{
  c::Clib, 
  intern::{intern, Interned}, 
  scan::stmt::LocalMod,
  primitive::litr::Litr
};
use crate::runtime::{calc::CalcRef, Scope};

pub type NativeFn = fn(Vec<CalcRef>, Scope)-> Litr;
pub type NativeMethod = fn(&mut NativeInstance, args:Vec<CalcRef>, Scope)-> Litr;

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
  pub getter: fn(&NativeInstance, get:Interned)-> Litr,
  pub setter: fn(&mut NativeInstance, set:Interned, to:Litr),
  pub index_get: fn(&NativeInstance, CalcRef)-> Litr,
  pub index_set: fn(&mut NativeInstance, CalcRef, Litr),
  pub next: fn(&mut NativeInstance)-> Litr,
  pub to_str: fn(&NativeInstance)-> String,
  pub onclone: fn(&NativeInstance)-> NativeInstance,
  pub ondrop: fn(&mut NativeInstance)
}

/// 传进main里的东西，用于原生模块向Key解释器传输模块内容
#[repr(C)]
struct NativeInterface {
  funcs: *mut Vec<(Interned, NativeFn)>,
  classes: *mut Vec<*mut NativeClassDef>
}

/// 传进premain的函数表, 保证原生模块能使用Key解释器上下文的函数
#[repr(C)]
struct PreMain {
  intern: fn(&[u8])-> Interned,
  err: fn(&str)-> !,
  find_var: fn(Scope, Interned)-> Option<CalcRef>,
}

/// 原生类型实例
#[derive(Debug)]
#[repr(C)]
pub struct NativeInstance {
  pub v: usize,
  pub w: usize,
  pub cls: *mut NativeClassDef,
}
impl Clone for NativeInstance {
  /// 调用自定义clone (key-native库中的默认clone行为也可用)
  fn clone(&self) -> Self {
    (unsafe{&*self.cls}.onclone)(self)
  }
}
impl Drop for NativeInstance {
  /// 调用自定义drop (key-native的默认drop不做任何事)
  fn drop(&mut self) {
    (unsafe{&*self.cls}.ondrop)(self)
  }
}

pub fn parse(path:&[u8])-> *const NativeMod {
  let lib = Clib::load(path);
  let mut m = Box::new(NativeMod {
    funcs: Vec::new(), classes: Vec::new()
  });
  unsafe {
    // 预备main, 将原生模块需要用的解释器的函数传过去
    // 没有extern前缀!
    let premain: fn(&PreMain) = std::mem::transmute(lib.get(b"premain").expect("模块需要'premain'函数初始化Key原生模块函数表"));
    premain(&PreMain {
      intern, err:|s|panic!("{}",s), find_var: Scope::var,
    });
    
    // 运行用户函数
    let main: fn(&mut NativeInterface) = std::mem::transmute(lib.get(b"main").expect("模块需要'main'函数作为主运行函数"));
    main(&mut NativeInterface {
      funcs: &mut m.funcs, classes: &mut m.classes
    });
  }
  Box::into_raw(m)
}
