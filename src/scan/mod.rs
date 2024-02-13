//! 将源码扫描为 AST的过程

use std::collections::HashMap;

use crate::ast::*;
use crate::intern::{
  intern,
  Interned
};
use crate::runtime::Scope;

mod charts;
mod stmt;
mod literal;
mod expr;

/// 将字符整理为ast
pub fn scan(src: Vec<u8>)-> Statements {
  // 已知此处所有变量未泄露
  // 为了规避&mut所有权检查，将引用改为指针
  let mut i = 0;
  let mut line = 1;
  let mut sttms = Statements::default();
  let mut scanner = Scanner {
    src:&*src, i:&mut i, line:&mut line,
    sttms:&mut sttms as *mut Statements
  };
  scanner.scan();
  sttms
}

struct Scanner<'a> {
  src: &'a [u8],
  i: *mut usize,
  line: *mut usize,
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
    unsafe{(*self.sttms).0.push((self.line(), s));}
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
  #[inline]
  fn line(&self)->usize {
    unsafe{*self.line}
  }

  /// 报错模板
  fn err(&self, s:&str)-> ! {
    panic!("{} 解析错误({})",s,self.line())
  }

  /// 跳过一段空格,换行符和注释
  fn spaces(&self) {
    let len = self.src.len();
    while self.i() < len {
      let c = self.cur();
      if c == b'\n' {
        unsafe{*self.line += 1;}
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
              unsafe{*self.line += 1;}
              self.next();
            }
            // 多行
            if nc == b'`' {
              self.set_i(next + 1);
              loop {
                self.next();
                if self.cur() == b'\n' {
                  unsafe{*self.line += 1;}
                }
                if self.cur() == b'`' {
                  let next = self.i() + 1;
                  if next >= len {
                    self.set_i(len);
                    return;
                  }
                  if self.src[next] == b'/' {
                    self.set_i(next + 1);
                    break;
                  }
                }
              }
            }
          }
        }
        _=> {
          break;
        }
      }
    }
  }

  /// 匹配标识符(如果匹配不到则返回的vec.len()为0)
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
        b'0'..=b'9' => {
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
    return Some(ident);
  }

  /// 检索一段 二元操作符
  fn operator(&self)-> &[u8] {
    // 如果第一个字符就是左括号就告诉Expr：这是个函数调用
    match self.cur() {
      // 这里不i+=1因为对应的解析函数会自动i+=1
      b'(' => return b"(",
      b'[' => return b"[",
      _=>{}
    }
    let mut i = self.i();
    let len = self.src.len();
    while i < len {
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
    return op;
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
      }else {self.err("类型声明不可为空")}
    }else {KsType::Any}
  }

  /// 解析函数声明的参数
  fn arguments(&self)-> Vec::<(Interned,KsType)> {
    self.spaces();
    let mut args = Vec::<(Interned,KsType)>::new();
    while let Some(n) = self.ident() {
      let arg = intern(n);
      let typ = self.typ();
      args.push((arg,typ));

      self.spaces();
      if self.cur() == b',' {
        self.next();
      }
      self.spaces();
    };
    args
  }
}


/// 语句方法
impl Scanner<'_> {
  /// 匹配一个语句
  fn stmt(&self)-> Stmt {
    stmt::stmt(self)
  }

  /// 从self.i直接开始解析一段表达式
  fn expr(&self)-> Expr {
    expr::expr(self)
  }

  /// 匹配一段表达式，传入二元表达式左边部分
  fn expr_with_left(&self, left:Expr)-> Expr {
    expr::with_left(self, left)
  }

  /// 匹配带括号的表达式(提升优先级和函数调用)
  fn expr_group(&self)-> Expr {
    expr::group(self)
  }

  /// 解析一段字面量
  /// 
  /// 同时解析一元运算符
  fn literal(&self)-> Expr {
    literal::literal(self)
  }
}
