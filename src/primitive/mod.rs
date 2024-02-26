//! 运行时提供的基础api
//! 
//! 和对基本类型方法的实现

use crate::native::{
  NativeClassDef, 
  NativeFn,
  NativeInstance
};
use crate::runtime::Class;
use crate::scan::literal::Litr;
use crate::intern::{Interned, intern};

pub mod std;

pub mod int;
pub mod sym;
pub mod obj;
pub mod iter;

fn getter(_v:*mut NativeInstance, _get:Interned)-> Litr {Litr::Uninit}
fn setter(_v:*mut NativeInstance, _set:Interned, _to:Litr) {}
fn igetter(_v:*mut NativeInstance, _get:usize)-> Litr {Litr::Uninit}
fn isetter(_v:*mut NativeInstance, _set:usize, _to:Litr) {}
fn onclone(v:*mut NativeInstance)-> NativeInstance {unsafe{&*v}.clone()}
fn ondrop(_v:*mut NativeInstance) {}

static mut CLASSES:Option<Vec<(Interned, NativeClassDef)>> = None;
fn new_class(s:&[u8], f:Vec<(Interned, NativeFn)>)-> (Interned, NativeClassDef) {
  let name = intern(s);
  (name, NativeClassDef {
    name,
    methods: Vec::new(),
    statics: f,
    getter, setter, igetter, isetter, onclone, ondrop
  })
}
pub fn classes()-> Vec<(Interned, Class)> {unsafe {
  if let Some(cls) = &mut CLASSES {
    cls.iter_mut().map(|(name, f)|(*name, Class::Native(f))).collect()
  }else {
    let obj_c = new_class(b"Obj", obj::statics());
    let sym_c = new_class(b"Sym", sym::statics());
    CLASSES = Some(vec![obj_c, sym_c]);
    classes()
  }
}}
