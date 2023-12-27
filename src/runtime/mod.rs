//! 运行时环境

use crate::ast::{
  Statements,
  Statmnt,
  Expr,
  Ident,
  KsType, 
  KsAssign, Imme, KsLocalFunc, Executable
};
use std::collections::HashMap;

mod io;

struct Var {
  p: usize, // pointer
  t: Ident  // type
}
#[derive(Debug)]
pub struct Scope {
  parent: Option<Box<Scope>>,
  types: HashMap<Ident, KsType>,
  vars: HashMap<Ident, Imme>
}
impl Scope {
  /// 在此作用域运行ast代码
  pub fn run(&self, codes:&Statements) {
    for sm in codes {
      self.evil(sm);
    }
  }

  /// 在作用域解析一个语句
  pub fn evil(&self, code:&Statmnt) {
    use Statmnt::*;
    match code {
      Expression(e)=> {
        match &**e {
          Expr::Call { args, targ }=> {
            self.call(args, targ);
          },
          _=> {}
        };
      }
      _=> {}
    }
  }

  /// 调用一个函数
  pub fn call(&self, args:&Vec<Box<Expr>>, targ: &Vec<u8>) {
    match self.var(targ) {
      Imme::Func(exec)=> {
        use Executable::*;

        let args_calced = args.iter().map(|e| self.calc(e)).collect();
        match exec {
          RTVoid(f)=> f(&args_calced),
          _=> {}
        }
      },
      _=> panic!("'{}' 不是一个函数 (运行时)", String::from_utf8_lossy(targ))
    }
  }

  /// 在作用域找一个变量
  pub fn var(&self, s:&Ident)-> &Imme {
    if let Some(v) = self.vars.get(s) {
      return v;
    }
    if let Some(v) = &self.parent {
      return &*v.var(s);
    }else {
      panic!("无法找到变量 '{}' (运行时)", String::from_utf8_lossy(s));
    }
  }

  /// 在此作用域计算表达式的值
  pub fn calc(&self, e:&Expr)-> Imme {
    use Expr::*;
    match e {
      Immediate(imme)=> {
        return imme.clone();
      }
      _=> panic!("算不出来 (运行时)")
    }
  }
}

/// 自定义此函数为脚本添加新函数和变量
pub fn top_scope()-> Scope {
  let types = HashMap::<Ident, KsType>::new();
  let mut vars = HashMap::<Ident, Imme>::new();
  vars.insert(b"print".to_vec(), 
    Imme::Func(Executable::RTVoid(io::print))
  );
  Scope {parent: None, types, vars}
}

