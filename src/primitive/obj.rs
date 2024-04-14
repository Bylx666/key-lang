//! Obj基本类型的静态方法

use crate::{
  intern::{intern, Interned}, 
  native::{NativeClassDef, NativeFn, NativeInstance}, 
  primitive::{litr::Litr, sym}, 
  runtime::{calc::CalcRef, Scope}
};
use std::collections::HashMap;

/// obj.keys()返回的迭代器类型
static mut ITER_KEYS:*mut NativeClassDef = std::ptr::null_mut();
/// obj.values()的迭代器
static mut ITER_VALUES:*mut NativeClassDef = std::ptr::null_mut();
/// obj.entries()的迭代器
static mut ITER_ENTRIES:*mut NativeClassDef = std::ptr::null_mut();

pub fn method(v:&mut HashMap<Interned, Litr>, scope:Scope, name:Interned, args:Vec<CalcRef>)-> Litr {
  match name.vec() {
    b"get"=> get(v, args),
    b"set"=> set(v, args),
    b"remove"=> remove(v, args),
    b"for_each"=> for_each(v, args, scope),
    b"has"=> has(v, args),
    b"keys"=> keys(v),
    b"values"=> values(v),
    b"entries"=> entries(v),
    b"len"=> Litr::Uint(v.len()),
    b"concat"=> concat(v, args),
    _=> panic!("Obj没有{}方法",name)
  }
}

/// 插入元素, 返回原有的元素或uninit
fn set(v:&mut HashMap<Interned, Litr>, args:Vec<CalcRef>)-> Litr {
  let name = match &**args.get(0).expect("obj.insert需要传入键名") {
    Litr::Str(s)=> intern(s.as_bytes()),
    _=> panic!("obj.insert第一个参数必须是字符串")
  };
  let elem = args.get(1).map_or(Litr::Uninit, |v|(**v).clone());
  v.insert(name, elem).unwrap_or(Litr::Uninit)
}

/// 删除一个元素,返回被删除的元素
fn remove(v:&mut HashMap<Interned, Litr>, args:Vec<CalcRef>)-> Litr {
  let name = match &**args.get(0).expect("obj.remove需要传入键名") {
    Litr::Str(s)=> intern(s.as_bytes()),
    _=> panic!("obj.remove第一个参数必须是字符串")
  };
  v.remove(&name).unwrap_or(Litr::Uninit)
}

/// 传入函数遍历|k,v|
fn for_each(v:&mut HashMap<Interned, Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("obj.for_each需要传入一个函数") {
    Litr::Func(f)=> f,
    _=> panic!("obj.for_each第一个参数必须是Func")
  };
  v.iter_mut().for_each(|(k,v)|{
    scope.call(vec![CalcRef::Own(Litr::Str(k.str())), CalcRef::Ref(v)], f);
  });
  Litr::Uninit
}

/// 获取Litr
fn get(v:&mut HashMap<Interned, Litr>, args:Vec<CalcRef>)-> Litr {
  let name = match &**args.get(0).expect("obj.get需要传入键名") {
    Litr::Str(s)=> intern(s.as_bytes()),
    _=> panic!("obj.get第一个参数必须是字符串")
  };
  v.get(&name).map_or(Litr::Uninit, |n|n.clone())
}

/// 测试是否有该元素
fn has(v:&mut HashMap<Interned, Litr>, args:Vec<CalcRef>)-> Litr {
  let name = match &**args.get(0).expect("obj.has需要传入键名") {
    Litr::Str(s)=> intern(s.as_bytes()),
    _=> panic!("obj.has第一个参数必须是字符串")
  };
  Litr::Bool(match v.get(&name) {
    Some(_)=> true,
    None=> false
  })
}

/// 返回对所有键名的迭代器
fn keys(o:&mut HashMap<Interned, Litr>)-> Litr {
  let v = Box::into_raw(Box::new(o.keys())) as usize;
  Litr::Ninst(NativeInstance {cls:unsafe{ITER_KEYS},v,w:0})
}

/// 返回对所有值的迭代器
fn values(o:&mut HashMap<Interned, Litr>)-> Litr {
  let v = Box::into_raw(Box::new(o.values())) as usize;
  Litr::Ninst(NativeInstance {cls:unsafe{ITER_VALUES},v,w:0})
}

/// 返回对所有键对的迭代器
fn entries(o:&mut HashMap<Interned, Litr>)-> Litr {
  let v = Box::into_raw(Box::new(o.iter())) as usize;
  Litr::Ninst(NativeInstance {cls:unsafe{ITER_ENTRIES},v,w:0})
}

/// concat内部使用
fn _concat_extend(o:&mut HashMap<Interned, Litr>, arg:&Litr) {
  match arg {
    Litr::Obj(other)=>
      o.extend(other.iter().map(|(k,v)|(*k, v.clone()))),
    Litr::Inst(inst)=> {
      let cls = unsafe {&*inst.cls};
      for (i,n) in cls.props.iter().enumerate() {
        if !n.public {continue;}
        o.insert(n.name, unsafe{inst.v.get_unchecked(i).clone()});
      }
    }
    _=> panic!("obj.concat的参数只能是Obj或实例")
  };
}

/// 将Obj和Obj或Inst合并(inst只会拼接public的属性)
fn concat(o:&mut HashMap<Interned, Litr>, args:Vec<CalcRef>)-> Litr {
  _concat_extend(o, &**args.get(0).expect("obj.concat需要传入拼接对象或实例"));
  Litr::Uninit
}


// - statics -
pub fn statics()-> Vec<(Interned, NativeFn)> {
  unsafe {
    use std::collections::hash_map::{Keys, Values, Iter};

    // 初始化keys()迭代器类
    ITER_KEYS = Box::into_raw(Box::new(super::new_iter_class(
      b"Obj.keys", 
      |v| {
        let itr = v.v as *mut Keys<'_, Interned, Litr>;
        (*itr).next().map_or(sym::iter_end(), 
        |v|Litr::Str(v.str()))
      }, 
      |v| {
        drop(Box::from_raw(v.v as *mut Keys<'_, Interned, Litr>))
      }
    )));

    // 初始化value()迭代器类
    ITER_VALUES = Box::into_raw(Box::new(super::new_iter_class(
      b"Obj.values", 
      |v| {
        let itr = v.v as *mut Values<'_, Interned, Litr>;
        (*itr).next()
          .map_or(sym::iter_end(),|v|v.clone())
      }, 
      |v| {
        drop(Box::from_raw(v.v as *mut Values<'_, Interned, Litr>))
      }
    )));

    // 初始化value()迭代器类
    ITER_ENTRIES = Box::into_raw(Box::new(super::new_iter_class(
      b"Obj.entries", 
      |v| {
        let itr = v.v as *mut Iter<'_, Interned, Litr>;
        (*itr).next().map_or(sym::iter_end(),|(k,v)|Litr::List(
          vec![Litr::Str(k.str()), v.clone()]
        ))
      }, 
      |v| {
        drop(Box::from_raw(v.v as *mut Iter<'_, Interned, Litr>))
      }
    )));
  }

  vec![
    (intern(b"concat"), s_concat),
    (intern(b"from_list"), s_from_list),
    (intern(b"group_by"), s_group_by),
    (intern(b"new"), s_new)
  ]
}

/// 将多个Obj或Inst拼接成一个Obj,前后顺序会影响覆盖关系
fn s_concat(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let mut o = HashMap::new();
  for arg in args.into_iter() {
    _concat_extend(&mut o, &*arg);
  }
  Litr::Obj(o)
}

// 通过成员全都为[key,value]的列表构造一个Obj
fn s_from_list(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let l = match &**args.get(0).expect("Obj::from_list需要传入一个List") {
    Litr::List(l)=> l,
    _=> panic!("Obj::from_list第一个参数必须是List")
  };
  let mut o = HashMap::with_capacity(l.len());
  for v in l {
    if let Litr::List(v) = v {
      let key = if let Some(s) = v.get(0) {
        if let Litr::Str(s) = s {
          intern(s.as_bytes())
        // 键对中如果第一个不是字符串就直接跳过
        }else {continue;}
      }else {continue;};
      let val = v.get(1).map_or(Litr::Uninit, |n|n.clone());
      o.insert(key, val);
    }
  }
  Litr::Obj(o)
}

/// 传入一个返回字符串的函数, 根据字符串把List的内容分类成Obj
fn s_group_by(args:Vec<CalcRef>, cx:Scope)-> Litr {
  assert!(args.len()>=2, "Obj::group_by需要一个List和返回字符串的函数");
  let mut args = args.into_iter();
  let mut o = HashMap::new();
  let mut ls_ = args.next().unwrap();
  let ls = if let Litr::List(l) = &mut *ls_ {l}
    else {panic!("Obj::group_by第一个参数必须是List")};
  let f_ = args.next().unwrap();
  let f = if let Litr::Func(f) = &*f_ {f}
    else {panic!("Obj::group_by第二个参数必须是返回字符串的Func")};
  
  for elem in ls.iter_mut() {
    let sort_str = cx.call(vec![CalcRef::Ref(elem)], &f);
    let s = intern(
      if let Litr::Str(s) = &sort_str { s.as_bytes() } 
      else {continue;}
    );
    
    match o.get_mut(&s) {
      Some(v)=> if let Litr::List(v) = v {
        v.push(elem.clone());
      }else {unreachable!()}
      None=> {
        o.insert(s, Litr::List(vec![elem.clone()]));
      }
    }
  }
  Litr::Obj(o)
}

/// 允许传入一个长度值作为其初始大小
fn s_new(args:Vec<CalcRef>,_cx:Scope)-> Litr {
  Litr::Obj(if let Some(n) = args.get(0) {
    HashMap::with_capacity(match &**n {
      Litr::Uint(n)=> *n,
      Litr::Int(n)=> *n as usize,
      _=> 0
    })
  }else {HashMap::new()})
}
