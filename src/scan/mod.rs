//! 将源码扫描为 AST的过程

use std::collections::HashMap;

use crate::intern::{
  intern,
  Interned
};
use crate::runtime::Scope;
use crate::LINE;

pub mod charts;
pub mod stmt;
pub mod literal;
pub mod expr;

use stmt::{Statements, Stmt};
use literal::{Litr, KsType, ArgDecl};
use expr::Expr;

/// 将字符扫描为ast
pub fn scan(src: &[u8])-> Statements {
  // 已知此处所有变量未泄露
  // 为了规避&mut所有权检查，将引用改为指针
  let mut i = 0;
  let mut sttms = Statements::default();
  let mut scanner = Scanner {
    src, i:&mut i, 
    sttms:&mut sttms as *mut Statements
  };
  scanner.scan();
  sttms
}

struct Scanner<'a> {
  src: &'a [u8],
  i: *mut usize,
  sttms: *mut Statements,
}


/// 通用方法
impl Scanner<'_> {
  /// 启动扫描
  fn scan(mut self) {
    let len = self.src.len();
    while self.i() < len {
      let s = self.stmt();
      if let Stmt::Empty = s {
        continue;
      }
      self.push(s);
    }
  }

  #[inline]
  fn push(&self, s:Stmt) {
    unsafe{(*self.sttms).0.push((LINE, s));}
  }
  /// 获取当前字符(ascii u8)
  #[inline]
  fn cur(&self)-> u8 {
    unsafe { *self.src.get_unchecked(*self.i) }
  }

  /// 使i += 1
  #[inline]
  fn next(&self) {
    unsafe{*self.i += 1;}
  }
  #[inline]
  fn i(&self)->usize {
    unsafe{*self.i}
  }
  #[inline]
  fn set_i(&self,n:usize) {
    unsafe{*self.i = n;}
  }

  /// 跳过一段空格,换行符和注释
  fn spaces(&self) {
    let len = self.src.len();
    loop {
      let c = self.cur();
      if c == b'\n' {
        unsafe{LINE += 1;}
      }
      match c {
        b'\n' | b'\r' | b' ' => {
          self.next();
        },
        // 解析注释
        b'/' => {
          let next = self.i() + 1;
          if next < len {
            let nc = self.src[next];
            // 单行
            if nc == b'/' {
              self.set_i(next + 1);
              while self.cur() != b'\n' {
                self.next();
                if self.i() >= len {
                  return;
                }
              }
              unsafe{LINE += 1;}
              self.next();
            }
            // 多行
            else if nc == b'\'' {
              self.set_i(next + 1);
              loop {
                self.next();
                if self.cur() == b'\n' {
                  unsafe{LINE += 1;}
                }
                if self.cur() == b'\'' {
                  let next = self.i() + 1;
                  if next >= len {
                    self.set_i(len);
                    return;
                  }
                  if self.src[next] == b'/' {
                    self.set_i(next + 1);
                  }
                }
              }
            }
            // /后面不是注释就直接返回
            else{return;}
          }
        }
        _=> break
      }
    }
  }

  /// 匹配标识符
  fn ident(&self)-> Option<&[u8]> {
    let mut i = self.i();
    let len = self.src.len();
    if i >= len {
      return None;
    }
    
    // 判断首字是否为数字
    let first = self.src[i];
    if first>=b'0' && first<=b'9' {return None;}

    while i < len {
      let s = self.src[i];
      match s {
        b'_' | b'$' | b'~' | b'@' |
        b'A'..=b'Z' | b'a'..=b'z' |
        b'0'..=b'9'|
        // utf8双字节以上编码都以0b10xxxxxx开头
        128..=255 => {
          i += 1;
        },
        _=> {
          break;
        }
      }
    }

    if self.i() == i {return None;}
    let ident = &self.src[self.i()..i];
    self.set_i(i);
    Some(ident)
  }

  /// 检索一段 二元操作符
  fn operator(&self)-> &[u8] {
    // 如果第一个字符就是左括号就告诉Expr：这是个函数调用
    match self.cur() {
      // 这里不i+=1因为对应的解析函数会自动i+=1
      b'(' => return b"(",
      b'[' => return b"[",
      b'i' => {
        if self.i() + 1 < self.src.len() && self.src[self.i()+1] == b's' {
          self.set_i(self.i() + 2);
          return b"is";
        }
      }
      _=>()
    }
    let mut i = self.i();
    let len = self.src.len();
    while i + 1 < len {
      let cur = self.src[i];
      match cur {
        b'%'|b'&'|b'*'|b'+'|b'-'|b'.'|b'/'|b'<'|b'>'|b'='|b'^'|b'|'|b':'=> {
          i += 1;
        }
        _=> break
      }
    }

    let op = &self.src[self.i()..i];
    self.set_i(i);
    op
  }
  
  /// 解析类型声明
  fn typ(&self)-> KsType {
    if self.cur() == b':' {
      self.next();
      if let Some(decl) = self.ident() {
        use KsType::*;
        match decl {
          b"Int"=>Int,
          b"Uint"=>Uint,
          b"Float"=>Float,
          b"Bool"=>Bool,
          b"Func"=>Func, 
          b"Str"=>Str,
          b"Buffer"=>Buffer,
          b"List"=>List,
          b"Obj"=>Obj,
          _=> Class(intern(decl))
        }
      }else {panic!("类型声明不可为空")}
    }else {KsType::Any}
  }

  /// 解析函数声明的参数
  fn arguments(&self)-> Vec::<ArgDecl> {
    self.spaces();
    let mut args = Vec::new();
    while let Some(n) = self.ident() {
      let name = intern(n);
      let t = self.typ();

      self.spaces();
      let default = if self.cur() == b'=' {
        self.next();
        self.spaces();
        if let Expr::Literal(def) = self.literal() {
          def
        }else {panic!("默认参数只允许字面量")}
      }else {Litr::Uninit};

      if self.cur() == b',' {
        self.next();
      }
      self.spaces();
      args.push(ArgDecl {name, t, default});
    };
    args
  }
}
