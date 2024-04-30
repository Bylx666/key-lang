//! 提供Native Module的接口

use std::sync::{Condvar, Mutex};

use crate::{
  c::Clib, 
  intern::{intern, Interned}, 
  primitive::{
    litr::{Instance, Litr}, 
    planet
  },
  runtime::{outlive::{self, LocalFunc}, Variant}
};
use crate::runtime::{calc::CalcRef, Scope};

pub type NativeFn = fn(Vec<CalcRef>, Scope)-> Litr;
pub type NativeMethod = fn(&mut NativeInstance, args:Vec<CalcRef>, Scope)-> Litr;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct NativeMod {
  pub funcs: Vec<(Interned, NativeFn)>,
  pub classes: Vec<*const NativeClassDef>
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct NativeClassDef {
  pub statics: Vec<(Interned, NativeFn)>,
  pub methods: Vec<(Interned, NativeMethod)>,
  pub name: Interned,
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
  classes: *mut Vec<*const NativeClassDef>
}

/// 传进premain的函数表, 保证原生模块能使用Key解释器上下文的函数
#[repr(C)]
struct FuncTable {
  intern: fn(&[u8])-> Interned,
  err: fn(&str)-> !,
  find_var: fn(Scope, Interned)-> Option<CalcRef>,
  let_var: fn(Scope, Interned, Litr),
  const_var: fn(Scope, Interned),
  using: fn(Scope, Interned, *const NativeClassDef),
  call_local: fn(&LocalFunc, Vec<Litr>)-> Litr,
  call_at: fn(Scope, *mut Litr, &LocalFunc, Vec<Litr>)-> Litr,
  get_self: fn(Scope)-> *mut Litr,
  get_parent: fn(Scope)-> Option<Scope>,
  outlive_inc: fn(Scope),
  outlive_dec: fn(Scope),
  symcls: fn()-> *mut NativeClassDef,
  wait_inc: fn(),
  wait_dec: fn(),
  planet_new: fn()-> (*mut planet::Planet, *mut NativeClassDef),
  planet_ok: fn(&mut planet::Planet, Litr),
  local_instance_clone: fn(&Instance)-> Instance,
  local_instance_drop: fn(&mut Instance),
}
static FUNCTABLE:FuncTable = FuncTable {
  intern, 
  err:|s|panic!("{}",s), 
  find_var: Scope::var,
  let_var: |mut cx, name, v|cx.vars.push(Variant {
    locked:false, name, v
  }), 
  const_var: |cx, name|cx.lock(name),
  using: |mut cx, name, cls| cx.class_uses.push((name, crate::runtime::Class::Native(cls))),
  call_local: |f, args| f.scope.call_local(f, args),
  call_at: |cx, kself, f, args|{
    let f = LocalFunc::new(f.ptr, cx);
    Scope::call_local_with_self(&f, args, kself)
  },
  get_self: |cx|cx.kself,
  get_parent: |cx|cx.parent,
  outlive_inc: outlive::increase_scope_count,
  outlive_dec: outlive::decrease_scope_count,
  symcls: ||unsafe{crate::primitive::sym::SYMBOL_CLASS},
  wait_inc, wait_dec,
  planet_new: ||(planet::rust_new(), unsafe{planet::PLANET_CLASS}),
  planet_ok: planet::rust_ok,
  local_instance_clone: <Instance as Clone>::clone,
  local_instance_drop: |v|unsafe{std::ptr::drop_in_place(v)},
};

/// 原生类型实例
#[derive(Debug, PartialEq)]
#[repr(C)]
pub struct NativeInstance {
  pub v: usize,
  pub w: usize,
  pub cls: *const NativeClassDef,
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

/// wait_inc和wait_dec的主线程阻塞器
pub static mut WAITING: Mutex<isize> = Mutex::new(0);
pub static mut WAITING_CVAR: Condvar = Condvar::new();

/// 增加占用数量
pub fn wait_inc() {
  unsafe{*WAITING.lock().unwrap() += 1}
}
/// 减少占用数量
pub fn wait_dec() {
  unsafe{
    *WAITING.lock().unwrap() -= 1;
    WAITING_CVAR.notify_one();
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
    let premain: fn(&FuncTable) = std::mem::transmute(lib.get(b"premain").expect("请为你的项目添加'key_native'库. 入门教程请参见: https://docs.subkey.top/native/1.start"));
    premain(&FUNCTABLE);
    
    // 运行用户函数
    let main: fn(&mut NativeInterface) = std::mem::transmute(lib.get(b"main").expect("需要为main函数添加符号链接'#[no_mangle]'. 入门教程请参见: https://docs.subkey.top/native/1.start"));
    main(&mut NativeInterface {
      funcs: &mut m.funcs, classes: &mut m.classes
    });
  }
  Box::into_raw(m)
}
