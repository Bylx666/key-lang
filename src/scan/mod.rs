//! 将源码扫描为 AST的过程

use std::collections::HashMap;

use crate::intern::{
  intern,
  Interned
};
use crate::primitive::litr::{ArgDecl, KsType, Litr, LocalFuncRawArg};
use crate::runtime::Scope;
use crate::LINE;

pub mod charts;
pub mod stmt;
pub mod literal;
pub mod expr;

use stmt::{Statements, Stmt};
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
    unsafe{(*self.sttms).v.push((LINE, s));}
  }
  /// 获取当前字符(ascii u8)
  #[inline]
  fn cur(&self)-> u8 {
    unsafe { *self.src.get(*self.i).expect("未闭合的括号") }
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
    // 跳过空格和换行符
    while self.i() < len {
      match self.cur() {
        b'\n'=> {
          unsafe {LINE += 1}
          self.next();
        }
        b'\r' | b' '=> self.next(),
        _=> break
      }
    }

    // 识别注释并跳过
    if self.i() + 1 >= len || self.cur()!=b'/' {return}
    match self.src[self.i() + 1] {
      b'/'=> {
        self.set_i(self.i()+2);
        while self.i() < len && self.cur() != b'\n' {
          self.next();
        }
        // 注释结束后继续跳过空格(顺便把上文的\n跳过)
        self.spaces();
      }
      b'\''=> {
        self.set_i(self.i()+2);
        while self.i() < len {
          match self.cur() {
            b'\n'=> unsafe{
              LINE += 1;
              self.next();
            },
            b'\''=> {
              let next_i = self.i() + 1;
              if next_i<len && self.src[next_i]==b'/' {
                self.set_i(next_i+1);
                break;
              }
            }
            _=> self.next()
          }
        }

        self.spaces();
      }
      _=>()
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
        b'_' | b'~' | b'@' |
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
          b"Buf"=>Buf,
          b"List"=>List,
          b"Obj"=>Obj,
          _=> Class(intern(decl))
        }
      }else {panic!("类型声明不可为空")}
    }else {KsType::Any}
  }

  /// 解析函数声明的参数
  fn arguments(&self)-> LocalFuncRawArg {
    self.spaces();

    // 使用自定义参数语法
    if self.cur() == b'[' {
      self.next();
      let id = intern(self.ident().expect("自定义参数需要指定自定义参数名"));
      self.spaces();
      assert!(self.cur()==b']', "自定义参数的']'丢失");
      self.next();
      return LocalFuncRawArg::Custom(id);
    }

    let mut args = Vec::new();
    while let Some(n) = self.ident() {
      let name = intern(n);
      let t = self.typ();

      self.spaces();
      let default = if self.cur() == b'=' {
        self.next();
        self.spaces();
        let e = self.expr();
        if let Expr::Empty = e {
          panic!("'='后未填写默认参数")
        }else {
          e
        }
      }else {Expr::Literal(Litr::Uninit)};

      if self.cur() == b',' {
        self.next();
      }
      self.spaces();
      args.push(ArgDecl {name, t, default});
    };
    LocalFuncRawArg::Normal(args)
  }
}
