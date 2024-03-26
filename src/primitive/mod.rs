//! 运行时提供的基础api
//! 
//! 和对基本类型方法的实现

pub mod litr;

pub mod kstd;

pub mod buf;
pub mod list;
pub mod int;
pub mod sym;
pub mod obj;
pub mod iter;
pub mod func;

use litr::Litr;
use crate::native::{
  NativeClassDef, 
  NativeFn,
  NativeInstance
};
use crate::runtime::{calc::CalcRef, Class, Scope};
use crate::intern::{Interned, intern};


fn getter(_v:&NativeInstance, _get:Interned)-> Litr {Litr::Uninit}
fn setter(_v:&mut NativeInstance, _set:Interned, _to:Litr) {}
fn index_get(_v:&NativeInstance, _get:CalcRef)-> Litr {Litr::Uninit}
fn index_set(_v:&mut NativeInstance, _set:CalcRef, _to:Litr) {}
fn next(_v:&mut NativeInstance)-> Litr {Litr::Uninit}
fn onclone(v:&NativeInstance)-> NativeInstance {unsafe{&*v}.clone()}
fn ondrop(_v:&mut NativeInstance) {}

static mut CLASSES:Option<Vec<(Interned, NativeClassDef)>> = None;

/// 创建一个只有静态方法的原生类
fn new_static_class(s:&[u8], f:Vec<(Interned, NativeFn)>)-> (Interned, NativeClassDef) {
  let name = intern(s);
  (name, NativeClassDef {
    name,
    methods: Vec::new(),
    statics: f,
    getter, setter,
    index_get, index_set,
    next, onclone, ondrop
  })
}

/// 创建一个只带有迭代器的原生类
fn new_iter_class(
  s:&[u8], 
  next:fn(&mut NativeInstance)-> Litr, 
  ondrop:fn(&mut NativeInstance)
)-> NativeClassDef {
  let name = intern(s);
  NativeClassDef {
    name,
    methods: Vec::new(),
    statics: Vec::new(),
    getter, setter,
    index_get, index_set,
    next, ondrop,
    onclone: |v|panic!("该迭代器{}无法复制. 请考虑用take函数代替", unsafe{&*v.cls}.name)
  }
}

pub fn classes()-> Vec<(Interned, Class)> {unsafe {
  if let Some(cls) = &mut CLASSES {
    cls.iter_mut().map(|(name, f)|(*name, Class::Native(f))).collect()
  }else {
    let buf_c = new_static_class(b"Buf", buf::statics());
    let list_c = new_static_class(b"List", list::statics());
    let obj_c = new_static_class(b"Obj", obj::statics());
    let sym_c = new_static_class(b"Sym", sym::statics());
    let func_c = new_static_class(b"Func", func::statics());
    CLASSES = Some(vec![buf_c, list_c, obj_c, sym_c, func_c]);
    classes()
  }
}}
