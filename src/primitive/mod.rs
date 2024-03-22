//! 运行时提供的基础api
//! 
//! 和对基本类型方法的实现

use crate::native::{
  NativeClassDef, 
  NativeFn,
  NativeInstance
};
use crate::runtime::{calc::CalcRef, Class, Scope};
use crate::scan::literal::Litr;
use crate::intern::{Interned, intern};

pub mod kstd;

pub mod buf;
pub mod int;
pub mod sym;
pub mod obj;
pub mod iter;
pub mod func;

fn getter(_v:*mut NativeInstance, _get:Interned)-> Litr {Litr::Uninit}
fn setter(_v:*mut NativeInstance, _set:Interned, _to:Litr) {}
fn index_get(_v:*mut NativeInstance, _get:CalcRef)-> Litr {Litr::Uninit}
fn index_set(_v:*mut NativeInstance, _set:CalcRef, _to:Litr) {}
fn next(_v:*mut NativeInstance)-> Litr {Litr::Uninit}
fn onclone(v:*mut NativeInstance)-> NativeInstance {unsafe{&*v}.clone()}
fn ondrop(_v:*mut NativeInstance) {}

static mut CLASSES:Option<Vec<(Interned, NativeClassDef)>> = None;

fn new_class(s:&[u8], f:Vec<(Interned, NativeFn)>)-> (Interned, NativeClassDef) {
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

pub fn classes()-> Vec<(Interned, Class)> {unsafe {
  if let Some(cls) = &mut CLASSES {
    cls.iter_mut().map(|(name, f)|(*name, Class::Native(f))).collect()
  }else {
    let buf_c = new_class(b"Buf", buf::statics());
    let obj_c = new_class(b"Obj", obj::statics());
    let sym_c = new_class(b"Sym", sym::statics());
    let func_c = new_class(b"Func", func::statics());
    CLASSES = Some(vec![buf_c, obj_c, sym_c, func_c]);
    classes()
  }
}}

/// 从args迭代器中获取下一个参数
macro_rules! next_arg {
  ($args:ident $($err:literal)+)=> {
    match $args.next() {
      Some(v)=> v,
      None=> panic!($($err,)+)
    }
  };
  ($args:ident $t:ty:$e:ident:$($t_err:literal)+; $($err:literal)+)=> {
    match $args.next() {
      Some(v)=> match v {
        Litr::$t(v)=> v,
        _=> panic!($($t_err,)+)
      },
      None=> panic!($($err,)+)
    }
  }
}
pub(self) use next_arg;
