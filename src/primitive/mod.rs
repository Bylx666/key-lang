//! 运行时提供的基础api
//! 
//! 和对基本类型方法的实现

pub mod litr;

pub mod kstd;

pub mod buf;
pub mod list;
pub mod int;
pub mod float;
pub mod kstr;
pub mod sym;
pub mod obj;
pub mod iter;
pub mod func;

use litr::{Litr, Function};
use crate::native::{
  NativeClassDef, 
  NativeFn,
  NativeInstance
};
use crate::runtime::{calc::CalcRef, Scope, Class};
use crate::intern::{Interned, intern};

static mut CLASSES:Option<Vec<(Interned, NativeClassDef)>> = None;

pub fn ninst_to_str(inst:&NativeInstance)-> String {
  format!("{} {{ Builtin }}", &unsafe{&*inst.cls}.name.str())
}

/// 创建一个只有静态方法的原生类
fn new_static_class(s:&[u8], f:Vec<(Interned, NativeFn)>)-> (Interned, NativeClassDef) {
  let name = intern(s);
  (name, NativeClassDef {
    name,
    methods: Vec::new(),
    statics: f,
    getter:|_,_|Litr::Uninit,
    setter:|_,_,_|(),
    index_get:|_,_|Litr::Uninit, 
    index_set:|_,_,_|(),
    to_str: ninst_to_str,
    next:|_|Litr::Sym(sym::Symbol::IterEnd), 
    onclone:|v|v.clone(), 
    ondrop:|_|()
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
    getter:|_,_|Litr::Uninit, 
    setter:|_,_,_|(),
    index_get:|_,_|Litr::Uninit, 
    index_set:|_,_,_|(),
    next, 
    to_str: ninst_to_str,
    ondrop,
    onclone: |v|panic!("该迭代器{}无法复制. 请考虑用take函数代替", unsafe{&*v.cls}.name)
  }
}

/// 返回只含有静态函数的内置类
pub fn classes()-> Vec<(Interned, Class)> {unsafe {
  if let Some(cls) = &mut CLASSES {
    cls.iter_mut().map(|(name, f)|(*name, Class::Native(f))).collect()
  }else {
    CLASSES = Some(vec![
      new_static_class(b"Buf", buf::statics()),
      new_static_class(b"List", list::statics()),
      new_static_class(b"Obj", obj::statics()),
      new_static_class(b"Int", int::statics_int()),
      new_static_class(b"Uint", int::statics_uint()),
      new_static_class(b"Float", float::statics()),
      new_static_class(b"Str", kstr::statics()),
      new_static_class(b"Sym", sym::statics()),
      new_static_class(b"Func", func::statics()),
    ]);
    classes()
  }
}}


/// 在作用域中获取Litr的属性
pub fn get_prop(this:Scope, mut from:CalcRef, find:Interned)-> CalcRef {
  match &mut *from {
    // 本地class的实例
    Litr::Inst(inst)=> {
      let can_access_private = unsafe {(*inst.cls).cx.exports} == this.exports;
      let cls = unsafe {&*inst.cls};

      // 寻找属性
      let props = &cls.props;
      for (n, prop) in props.iter().enumerate() {
        if prop.name == find {
          assert!(prop.public || can_access_private,
            "'{}'类型的成员属性'{}'是私有的", cls.name, find);
          return CalcRef::Ref(&mut inst.v[n]);
        }
      }

      panic!("'{}'类型上没有'{}'属性", cls.name, find)
    },

    // 原生类的实例
    Litr::Ninst(inst)=> {
      let cls = unsafe {&*inst.cls};
      CalcRef::Own((cls.getter)(inst, find))
    }

    // 哈希表
    Litr::Obj(map)=> map.get_mut(&find)
      .map_or(CalcRef::uninit(), |r|CalcRef::Ref(r)),

    // 以下都是对基本类型的getter行为
    Litr::Bool(v)=> CalcRef::Own(match find.vec() {
      b"rev"=> Litr::Bool(!*v),
      _=> Litr::Uninit
    }),

    Litr::Buf(v)=> CalcRef::Own(match find.vec() {
      b"len"=> Litr::Uint(v.len()),
      b"ptr"=> Litr::Uint(v.as_mut_ptr() as usize),
      b"capacity"=> Litr::Uint(v.capacity()),
      _=> Litr::Uninit
    }),

    Litr::Func(f)=> CalcRef::Own(match find.vec() {
      b"type"=> match f {
        Function::Local(_)=> Litr::Str("local".to_owned()),
        Function::Extern(_)=> Litr::Str("extern".to_owned()),
        Function::Native(_)=> Litr::Str("native".to_owned())
      }
      b"raw"=> match f {
        Function::Local(f)=> Litr::Uint(f.ptr as _),
        Function::Native(f)=> Litr::Uint(*f as usize),
        Function::Extern(e)=> Litr::Uint(e.ptr)
      }
      _=> Litr::Uninit
    }),

    Litr::List(v)=> CalcRef::Own(match find.vec() {
      b"len"=> Litr::Uint(v.len()),
      b"capacity"=> Litr::Uint(v.capacity()),
      _=> Litr::Uninit
    }),

    Litr::Str(s)=> CalcRef::Own(match find.vec() {
      b"ptr"=> Litr::Uint(s.as_mut_ptr() as _),
      b"byte_len"=> Litr::Uint(s.len()),
      b"len"=> Litr::Uint(s.chars().count()),
      b"lines"=> Litr::Uint(s.lines().count()),
      b"capacity"=> Litr::Uint(s.capacity()),
      _=> Litr::Uninit
    }),

    Litr::Int(n)=> CalcRef::Own(match find.vec() {
      b"int"=> Litr::Int(*n),
      b"uint"=> Litr::Uint(*n as _),
      b"float"=> Litr::Float(*n as _),
      _=> Litr::Uninit
    }),

    Litr::Uint(n)=> CalcRef::Own(match find.vec() {
      b"int"=> Litr::Int(*n as _),
      b"uint"=> Litr::Uint(*n),
      b"float"=> Litr::Float(*n as _),
      _=> Litr::Uninit
    }),

    Litr::Float(n)=> CalcRef::Own(match find.vec() {
      b"int"=> Litr::Int(*n as _),
      b"uint"=> Litr::Uint(*n as _),
      b"float"=> Litr::Float(*n),
      _=> Litr::Uninit
    }),

    _=> CalcRef::uninit()
  }
}
