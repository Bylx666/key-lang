//! 运行时环境
//! 将解析的ast放在实际作用域中运行

use crate::ast::{
  Statements,
  Stmt,
  Expr,
  Ident,
  KsType, 
  Litr,
  KsLocalFunc,
  Executable,
  KsCall
};
use std::collections::HashMap;

mod io;

struct Var {
  p: usize, // pointer
  t: Ident  // type
}

/// 一个运行时作用域
/// run函数需要mut因为需要跟踪行数
#[derive(Debug)]
pub struct Scope {
  parent: Option<Box<Scope>>,
  types: HashMap<Ident, KsType>,
  vars: HashMap<Ident, Litr>,
  line: usize
}
impl Scope {
  /// 在此作用域运行ast代码
  pub fn run(&mut self, codes:&Statements) {
    for (l, sm) in &codes.exec {
      self.line = *l;
      self.evil(sm);
    }
  }

  /// 在作用域解析一个语句
  pub fn evil(&mut self, code:&Stmt) {
    use Stmt::*;
    match code {
      Expression(e)=> {
        if let Expr::Call(call)= &**e {
          self.call(call);
        }
      }
      Let(a)=> {
        let v = self.calc(&a.val);
        self.set_var(a.id.clone(), v);
      }
      _=> {}
    }
  }

  /// 调用一个函数
  pub fn call(&self, call: &Box<KsCall>) {
    let targ = self.calc(&call.targ);
    let args = self.calc(&call.args);
    if let Litr::Func(exec) = targ {
      use Executable::*;
      match &*exec {
        RTVoid(f)=> f(&args),
        _=> {}
      }
    }
    else {self.err(&format!("'{:?}' 不是一个函数", targ))}
  }

  /// 在作用域找一个变量
  pub fn var(&self, s:&Ident)-> Litr {
    if let Some(v) = self.vars.get(s) {
      return v.clone();
    }
    if let Some(v) = &self.parent {
      return v.var(s).clone();
    }else {
      self.err(&format!("无法找到变量 '{}'", String::from_utf8_lossy(s)));
    }
  }

  pub fn set_var(&mut self, s:Ident, v:Litr) {
    let old = self.vars.insert(s, v);
    drop(old);
  }

  /// 在此作用域计算表达式的值
  /// 会将变量计算成实际值
  pub fn calc(&self, e:&Expr)-> Litr {
    use Expr::*;
    match e {
      Literal(litr)=> {
        let ret = if let Litr::Variant(id) = litr {
          self.var(id)
        }else {
          litr.clone()
        };
        return ret;
      }
      Binary(bin)=> {
        let left = if let Literal(Litr::Variant(id)) = &bin.left {
          self.var(id)
        }else {
          self.calc(&bin.left)
        };
        let right = if let Literal(Litr::Variant(id)) = &bin.right {
          self.var(id)  
        }else {
          self.calc(&bin.right)
        };

        use Litr::*;
        match &*bin.sym {

          b"+" => {
            match (left, right) {
              (Int(l),Int(r))=> Int(l+r),
              (Uint(l),Uint(r))=> Uint(l+r),
              (Float(l),Float(r))=> Float(l+r),
              _=> self.err("相加类型不同")
            }
          }
          b"-" => {
            match (left, right) {
              (Int(l),Int(r))=> Int(l-r),
              _=> self.err("相减类型不同")
            }
          }
          b"*" => {
            match (left, right) {
              (Int(l),Int(r))=> Int(l*r),
              _=> self.err("相乘类型不同")
            }
          }
          b"/" => {
            match (left, right) {
              (Int(l),Int(r))=> Int(l/r),
              _=> self.err("相除类型不同")
            }
          }

          // 解析,运算符
          b"," => {
            match left {
              Array(mut o)=> {
                o.push(right);
                Array(o)
              }
              _=> {
                Array(Box::new(vec![left, right]))
              }
            }
          }
          _=> self.err(&format!("非法运算符'{}'", String::from_utf8_lossy(&bin.sym)))
        }
      }
      _=> self.err("算不出来 ")
    }
  }

  fn err(&self, s:&str)-> ! {
    panic!("{} 运行时({})", s, self.line)
  }
}



/// 自定义此函数为脚本添加新函数和变量
pub fn top_scope()-> Scope {
  let types = HashMap::<Ident, KsType>::new();
  let mut vars = HashMap::<Ident, Litr>::new();
  vars.insert(b"print".to_vec(), 
    Litr::Func(Box::new(Executable::RTVoid(io::print)))
  );
  Scope {parent: None, types, vars, line:0}
}

