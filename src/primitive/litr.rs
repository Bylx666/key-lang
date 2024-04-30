//! Ks值 Litr的定义和运算符

use std::collections::HashMap;
use crate::{
  intern::{intern, Interned}, native::NativeInstance, runtime::Scope, scan::{expr::Expr, stmt::{ClassDef, Statements}}
};

pub use crate::runtime::outlive::LocalFunc;

#[derive(Clone, Debug)]
pub enum Litr {
  Uninit,

  Int    (isize),
  Uint   (usize),
  Float  (f64),
  Bool   (bool),

  Func   (Function), 
  Str    (String),
  Buf    (Vec<u8>),
  List   (Vec<Litr>),
  Obj    (HashMap<Interned, Litr>),
  Inst   (Instance),
  Ninst  (NativeInstance)
}

impl Litr {
  /// 由Key编译器提供的转字符
  pub fn str(&self)-> String {
    use Litr::*;
    match self {
      Uninit => String::default(),
      Int(n)=> n.to_string(),
      Uint(n)=> n.to_string(),
      Float(n)=> n.to_string(),
      Bool(n)=> n.to_string(),
      Func(f)=> {
        match f {
          Function::Local(f)=> format!("<Function {}>", f.name),
          Function::Extern(_)=> "<Extern Function>".to_string(),
          Function::Native(_)=> "<Native Function>".to_string()
        }
      }
      Str(s)=> s.clone(),
      List(a) => {
        let mut iter = a.iter();
        let mut str = String::new();
        str.push_str("[");
        if let Some(v) = iter.next() {
          str.push_str(&v.str());
        };
        while let Some(v) = iter.next() {
          str.push_str(", ");
          str.push_str(&v.str());
        }
        str.push_str("]");
        str
      },
      Buf(b)=> format!("Buf{:02X?}",b),
      Obj(map)=> {
        let mut s = String::new();
        s.push_str("{ ");
        let mut itr = map.iter();
        macro_rules! next {($k:ident,$v:ident)=>{{
          s.push_str(&$k.str());
          let v = $v.str();
          if v != "" {
            s.push_str(": ");
            s.push_str(&v);
          }
        }}}
        if let Some((k,v)) = itr.next() {
          next!(k,v)
        }
        while let Some((k, v)) = itr.next() {
          s.push_str(", ");
          next!(k,v)
        }
        s.push_str(" }");
        s
      },
      Inst(i)=> {
        let cls = unsafe{&*i.cls};
        let mut name = cls.props.iter();
        let mut val = i.v.iter();
        let mut s = String::new();
        macro_rules! next {($p:ident) => {{
          s.push_str(&$p.name.str());
          let next_v = val.next().unwrap().str();
          if next_v != "" {
            s.push_str(": ");
            s.push_str(&next_v);
          }
        }}}
        
        s.push_str(&cls.name.str());
        s.push_str(" { ");
        if let Some(p) = name.next() {
          next!(p);
        }
        for p in name {
          s.push_str(", ");
          next!(p);
        }
        s.push_str(" }");
        s
      }
      Ninst(inst)=> (unsafe { &*inst.cls }.to_str)(inst)
    }
  }
}

impl std::fmt::Display for Litr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.str())
  }
}
impl Default for Litr {
  fn default() -> Self {
    Litr::Uninit
  }
}

/// 针对函数的枚举
#[derive(Debug, Clone)]
pub enum Function {
  // Native模块或Runtime提供的Rust函数
  Native(crate::native::NativeFn),
  // 脚本定义的本地函数
  Local(LocalFunc),
  // 使用extern语句得到的C函数
  Extern(ExternFunc)
}

/// 参数声明
#[derive(Debug, Clone)]
pub struct ArgDecl {
  pub name: Interned,
  pub t: KsType,
  pub default: Expr
}

/// 未绑定作用域的本地定义函数
#[derive(Debug, Clone)]
pub struct LocalFuncRaw {
  pub name: Interned,
  pub argdecl: LocalFuncRawArg, 
  pub stmts: Statements
}

/// 本地函数传参方式
#[derive(Debug, Clone)]
pub enum LocalFuncRawArg {
  /// 正常按参数名传参
  Normal(Vec<ArgDecl>),
  /// 传入List由用户自由解析
  Custom(Interned)
}

/// 插件只有一个Native类型
#[derive(Debug, Clone)]
pub struct ExternFunc {
  pub argdecl: Vec<ArgDecl>, 
  pub ptr: *const (),
}

/// 类实例
#[derive(Debug)]
pub struct Instance {
  pub cls: *const ClassDef,
  pub v: Box<[Litr]>
}

impl Clone for Instance {
  /// 为想要管理内存的实例提供@clone方法
  fn clone(&self) -> Self {
    let fname = intern(b"@clone");
    let opt = unsafe{&*self.cls}.methods.iter().find(|f|f.f.name==fname);
    let cloned = Instance { cls: self.cls.clone(), v: self.v.clone() };
    match opt {
      Some(cls_f)=> {
        let f = LocalFunc::new(&cls_f.f, unsafe{&*self.cls}.cx);
        let res = Scope::call_local_with_self(&f, vec![], &mut Litr::Inst(cloned));
        if let Litr::Inst(v) = res {
          v
        }else {
          panic!("'{}'的@clone方法必须返回实例", cls_f.f.name);
        }
      }
      None=> cloned
    }
  }
}

impl Drop for Instance {
  /// 调用自定义drop
  fn drop(&mut self) {
    let fname = intern(b"@drop");
    let opt = unsafe{&*self.cls}.methods.iter().find(|f|f.f.name==fname);
    match opt {
      Some(cls_f)=> {
        let f = LocalFunc::new(&cls_f.f, unsafe{&*self.cls}.cx);
        // 不要额外调用clone
        let binding = &mut *std::mem::ManuallyDrop::new(Litr::Inst(Instance { cls: self.cls, v: self.v.clone() }));
        Scope::call_local_with_self(&f, vec![], binding);
      }
      None=> ()
    }
  }
}


/// Key语言内的类型声明
#[derive(Clone)]
pub enum KsType {
  Any,
  Int,
  Uint,
  Float,
  Bool,
  Func, 
  Str,
  Buf,
  List,
  Obj,
  Class(Interned)
}
impl KsType {
  /// 在一个作用域判断这个Litr是不是该类型
  pub fn is(&self, arg:&Litr, cx: crate::runtime::Scope)-> bool {
    use crate::runtime::Class;
    // 类型检查
    macro_rules! matcher {($($t:ident)*)=> {
      if let Litr::Uninit = &arg {
        if let KsType::Any = self {
          true
        }else { false }
      } else {match self {
        KsType::Any=> true,
        $(
          KsType::$t=> matches!(arg, Litr::$t(_)),
        )*
        KsType::Class(cls)=> {
          let cls = cx.find_class(*cls).unwrap_or_else(||panic!("无法找到'{}'类型",cls));
          match cls {
            Class::Local(cls)=> if let Litr::Inst(inst) = &arg {
              inst.cls == cls
            }else {false}
            Class::Native(cls)=> if let Litr::Ninst(inst) = &arg {
              inst.cls == cls
            }else {false}
          }
        }
      }}
    }}
    matcher!{Buf Bool Float Func Int Uint List Obj Str}
  }
}

impl std::fmt::Debug for KsType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    macro_rules! m {{$($t:ident)*}=> {
      match self {
        $(
          KsType::$t=> stringify!($t),
        )*
        KsType::Class(n)=> unsafe{ std::str::from_utf8_unchecked(n.vec()) }
      }
    }}
    f.write_str(m!{Any Buf Bool Float Func Int Uint List Obj Str})
  }
}


impl PartialEq for Litr {
  fn eq(&self, other: &Self) -> bool {
    match self {
      Litr::Ninst(l)=> if let Litr::Ninst(r) = other {l == r}else {false},
      Litr::Func(l)=> if let Litr::Func(r) = other {
        match (l, r) {
          (Function::Local(l), Function::Local(r))=> 
            l.ptr==r.ptr&&l.scope.ptr==r.scope.ptr,
          (Function::Native(l), Function::Native(r))=> l==r,
          _=> false
        }
      }else {false},
      Litr::Obj(l)=> if let Litr::Obj(r) = other {l == r}else {false},
      _=> self.partial_cmp(other) == Some(std::cmp::Ordering::Equal)
    }
  }
}

impl PartialOrd for Litr {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    use Litr::*;
    use std::cmp::Ordering::{self, *};

    /// 匹配单体
    fn match_basic(l:&Litr,r:&Litr)-> Option<Ordering> {
      match (l, r) {
        (Uninit, Uninit)=> Some(Equal),
        (Uint(l),Uint(r))=> l.partial_cmp(r),
        (Uint(l),Int(r))=> l.partial_cmp(&(*r as usize)),
        (Uint(l),Float(r))=> (*l as f64).partial_cmp(r),
        (Int(l), Uint(r))=> l.partial_cmp(&(*r as isize)),
        (Int(l), Int(r))=> l.partial_cmp(r),
        (Int(l), Float(r))=> (*l as f64).partial_cmp(r),
        (Float(l), Uint(r))=> l.partial_cmp(&(*r as f64)),
        (Float(l), Int(r))=> l.partial_cmp(&(*r as f64)),
        (Float(l), Float(r))=> l.partial_cmp(r),
        (Bool(l), Bool(r))=> l.partial_cmp(r),
        (Str(l), Str(r))=> l.partial_cmp(r),
        (Buf(l), Buf(r))=> l.partial_cmp(r),
        (List(l), List(r))=> match_list(l,r),
        (Obj(_), Obj(_))=> None,
        (Inst(l),Inst(r))=> {
          if l.cls==r.cls {
            match_list(&*l.v, &*r.v)
          }else {None}
        }
        _=> None
      }
    }

    /// 匹配多个
    fn match_list(l:&[Litr], r:&[Litr])-> Option<Ordering> {
      let len_matched = l.len().cmp(&r.len());
      if len_matched!=Equal {
        Some(len_matched)
      }else {
        let len = l.len();
        for i in 0..len {
          match match_basic(&l[i],&r[i]) {
            Some(Equal)=> (),
            n=> return n
          }
        }
        Some(Equal)
      }
    }

    match_basic(self,other)
  }
}