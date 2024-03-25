use std::cell::UnsafeCell;

use crate::{
  intern::intern, 
  native::NativeMethod, 
  runtime::calc::CalcRef,
  primitive::litr::{Litr, LocalFunc}
};

use super::sym::Symbol;

/// instance类型专用的迭代器
struct InstanceIter<'a> {
  f: &'a LocalFunc,
  kself: &'a mut Litr
}
impl Iterator for InstanceIter<'_> {
  type Item = Litr;
  fn next(&mut self) -> Option<Self::Item> {
    let r = self.f.scope.call_local_with_self(self.f, vec![], self.kself);
    if let Litr::Sym(s) = &r {
      if let Symbol::IterEnd = s {
        return None;
      }
    }
    Some(r)
  }
}

pub struct LitrIterator<'a> {
  inner: Box<dyn Iterator<Item = Litr> + 'a>
}
impl<'a> LitrIterator<'a> {
  pub fn new(v:&'a mut Litr)-> Self {
    let inner:Box<dyn Iterator<Item = Litr>> = match v {
      Litr::Str(s)=> Box::new(s.chars().map(|c|Litr::Str(c.to_string()))),
      Litr::Buf(v)=> Box::new(v.iter().map(|n|Litr::Uint((*n) as usize))),
      Litr::Uint(n)=> Box::new((0..*n).into_iter().map(|n|Litr::Uint(n))),
      Litr::Int(n)=> Box::new((0..*n).into_iter().map(|n|Litr::Int(n))),
      Litr::List(v)=> Box::new(v.iter().cloned()),
      Litr::Inst(inst)=> {
        let f = & unsafe{&*inst.cls}.methods.iter()
          .find(|f|f.name == intern(b"@next"))
          .expect("迭代class需要定义'.@next()'方法").f;
        Box::new(InstanceIter { f, kself:v })
      }
      Litr::Uninit => todo!(),
      Litr::Float(_) => todo!(),
      Litr::Bool(_) => todo!(),
      Litr::Func(_) => todo!(),
      Litr::Obj(_) => todo!(),
      Litr::Ninst(_) => todo!(),
      Litr::Sym(_) => todo!(),
      // Litr::Ninst(inst)=> {
      //   let next = unsafe {(*inst.cls).next};
      //   let res = next(inst);
      //   if let Litr::Sym(s) = &res {
      //     if let Symbol::IterEnd = s {
      //       return None;
      //     }
      //   }
      //   Some(res)
      // },
      // Litr::Obj(o)=> {
      //   o.iter()
      // },
      // Litr::Bool(_)=> panic!("Bool无法迭代"),
      // Litr::Float(_)=> panic!("Float无法迭代"),
      // Litr::Func(_)=> panic!("Func无法迭代"),
      // Litr::Uninit=> panic!("uninit作为迭代器要判死刑的"),
      // Litr::Sym(_)=> panic!("Sym无法迭代")
    };
    LitrIterator { inner }
  }
}

impl<'a> Iterator for LitrIterator<'a> {
  type Item = Litr;
  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next()
  }
}
