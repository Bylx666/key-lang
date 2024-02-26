use std::cell::UnsafeCell;

use crate::{intern::intern, native::NativeMethod, runtime::calc::CalcRef, scan::literal::{Litr, LocalFunc}};

use super::sym::Symbol;


pub struct LitrIterator<'a> {
  v: &'a mut Litr,
  n: usize
}
impl<'a> LitrIterator<'a> {
  pub fn new(v:&'a mut Litr)-> Self {
    LitrIterator { v, n: 0 }
  }
}

impl Iterator for LitrIterator<'_> {
  type Item = Litr;
  fn next(&mut self) -> Option<Self::Item> {
    match self.v {
      Litr::Str(s)=>unsafe{s.get_unchecked(self.n..).chars().next().map(|c|{
        let s = String::from(c);
        self.n += s.len();
        Litr::Str(s)}
      )}
      Litr::Buffer(v)=> {
        let v = v.get(self.n).map(|n|Litr::Uint((*n) as usize));
        self.n += 1;
        v
      }
      Litr::Uint(n)=> if self.n < *n {
        let v = Some(Litr::Uint(self.n));
        self.n += 1;
        v
      }else {None}
      Litr::Int(n)=> if (self.n as isize) < *n {
        let v = Some(Litr::Int(self.n as isize));
        self.n += 1;
        v
      }else {None}
      Litr::List(v)=> {
        let v = v.get(self.n).cloned();
        self.n += 1;
        v
      }
      Litr::Inst(inst)=> {
        // 没有next函数就找
        if self.n == 0 {
          let next_f = unsafe{&*inst.cls}.methods.iter()
            .find(|f|f.name == intern(b"@next"));
          self.n = match next_f {
            Some(f)=> {
              let mut f = Box::new(f.f.clone());
              f.bound = Some(Box::new(CalcRef::Ref(self.v)));
              Box::into_raw(f) as usize
            },
            None=> panic!("迭代class需要定义.@next()方法")
          };
        }
        let next = unsafe { &*(self.n as *const LocalFunc) };
        let res = next.scope.call_local(next, vec![]);
        if let Litr::Sym(s) = &res {
          if let Symbol::IterEnd = s {
            return None;
          }
        }
        Some(res)
      }
      Litr::Ninst(inst)=> {
        if self.n == 0 {
          let next_f = unsafe{&*inst.cls}.methods.iter()
            .find(|f|f.0 == intern(b"@next"));
          self.n = match next_f {
            Some(f)=> f.1 as usize,
            None=> panic!("迭代class需要定义.@next()方法")
          };
        }
        let next:NativeMethod = unsafe{std::mem::transmute(self.n)};
        let res = next(inst, vec![]);
        if let Litr::Sym(s) = &res {
          if let Symbol::IterEnd = s {
            return None;
          }
        }
        Some(res)
      },
      Litr::Bool(_)=> panic!("Bool无法迭代"),
      Litr::Float(_)=> panic!("Float无法迭代"),
      Litr::Func(_)=> panic!("Func无法迭代"),
      Litr::Uninit=> panic!("uninit作为迭代器要判死刑的"),
      Litr::Obj(_)=> panic!("Obj是无序的,难以迭代"),
      Litr::Sym(_)=> panic!("Sym无法迭代")
    }
  }
}

impl Drop for LitrIterator<'_> {
  fn drop(&mut self) {
    if let Litr::Inst(inst) = self.v {
      if self.n != 0 {
        unsafe {std::ptr::drop_in_place(self.n as *mut LocalFunc)}
      }
    }
  }
}
