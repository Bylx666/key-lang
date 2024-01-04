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

        /// 二进制运算中普通数字的戏份
        macro_rules! impl_num {
          ($pan:literal $op:tt) => {{
            match (left, right) {
              (Int(l),Int(r))=> Int(l $op r),
              (Uint(l),Uint(r))=> Uint(l $op r),
              (Float(l),Float(r))=> Float(l $op r),
              (Byte(l), Byte(r))=> Byte(l $op r),
              _=> self.err($pan)
            }
          }};
        }

        /// 二进制运算中无符号数的戏份
        macro_rules! impl_unsigned {
          ($pan:literal $op:tt) => {{
            match (left, right) {
              (Uint(l), Byte(r))=> Uint(l $op r as usize),
              (Uint(l), Uint(r))=> Uint(l $op r),
              (Uint(l), Int(r))=> Uint(l $op r as usize),
              (Byte(l), Byte(r))=> Byte(l $op r),
              (Byte(l), Uint(r))=> Byte(l $op r as u8),
              (Byte(l), Int(r))=> Byte(l $op r as u8),
              _=> self.err($pan)
            }
          }};
        }

        use Litr::*;
        match &*bin.op {
          // 数字
          b"+" => impl_num!("相加类型不同" +),
          b"-" => impl_num!("相减类型不同" -),
          b"*" => impl_num!("相乘类型不同" *),
          b"%" => impl_num!("求余类型不同" %),
          b"/" => {
            if match right {
              Int(r) => r == 0,
              Uint(r) => r == 0,
              Float(r) => r == 0.0,
              _=> false
            } {self.err("除数必须非0")}
            impl_num!("相除类型不同" /)
          }

          // usize
          b"<<" => impl_unsigned!("左移需要左值无符号" <<),
          b">>" => impl_unsigned!("右移需要左值无符号" >>),
          b"&" => impl_unsigned!("&需要左值无符号" &),
          b"^" => impl_unsigned!("^需要左值无符号" ^),
          b"|" => impl_unsigned!("|需要左值无符号" |),
          

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
          _=> self.err(&format!("非法运算符'{}'", String::from_utf8_lossy(&bin.op)))
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

