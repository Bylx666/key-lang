use std::cell::UnsafeCell;

use crate::scan::literal::Litr;


pub struct LitrIterator<'a> {
  pub v: &'a Litr,
  pub n: usize
}
impl<'a> LitrIterator<'a> {
  pub fn new(v:&'a Litr)-> Self {
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
      Litr::Bool(_)=> panic!("Bool无法迭代"),
      Litr::Float(_)=> panic!("Float无法迭代"),
      Litr::Func(_)=> panic!("Func无法迭代"),
      Litr::Inst(_)=> panic!("Inst?"),
      Litr::Ninst(_)=> panic!("Native Inst?"),
      Litr::Uninit=> panic!("uninit作为迭代器要判死刑的"),
      Litr::Obj(_)=> panic!("Obj是无序的,难以迭代"),
      Litr::Sym(_)=> panic!("Sym无法迭代")
    }
  }
}

