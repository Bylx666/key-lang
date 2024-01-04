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
        self.calc(e);
      }
      Let(a)=> {
        let v = self.calc(&a.val);
        self.let_var(a.id.clone(), v);
      }
      _=> {}
    }
  }

  /// 调用一个函数
  pub fn call(&mut self, call: &Box<KsCall>) {
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
  pub fn var(&self, s:&Ident)-> &Litr {
    if let Some(v) = self.vars.get(s) {
      return v;
    }
    if let Some(v) = &self.parent {
      return v.var(s);
    }
    self.err(&format!("无法找到变量 '{}'", String::from_utf8_lossy(s)));
  }

  pub fn modify_var(&mut self, s:&Ident, f:impl FnOnce(&mut Litr)) {
    if let Some(p) = self.vars.get_mut(s) {
      f(p);
      return;
    }
    if let Some(p) = &mut self.parent {
      return p.modify_var(s, f);
    }
    self.err(&format!("无法找到变量 '{}'", String::from_utf8_lossy(s)));
  }

  pub fn let_var(&mut self, s:Ident, v:Litr) {
    self.vars.insert(s, v);
  }


  /// 在此作用域计算表达式的值
  /// 会将变量计算成实际值
  pub fn calc(&mut self, e:&Expr)-> Litr {
    use Expr::*;
    match e {
      Call(c)=> {
        self.call(c);
        return Litr::Uninit;
      }
      Literal(litr)=> {
        let ret = if let Litr::Variant(id) = litr {
          self.var(id).clone()
        }else {
          litr.clone()
        };
        return ret;
      }
      Binary(bin)=> {
        let left = self.calc(&bin.left);
        let right = self.calc(&bin.right);

        /// 二元运算中普通数字的戏份
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

        /// 二元运算中无符号数的戏份
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

        /// 数字修改并赋值
        macro_rules! impl_num_assign {
          ($o:tt) => {{
            if let Expr::Literal(Variant(id)) = &bin.left {
              let line = self.line;
              let f = |p: &mut Litr|{
                // 数字默认为Int，所以所有数字类型安置Int自动转换
                let n = match (left, right.clone()) {
                  (Uint(l), Uint(r))=> Uint(l $o r),
                  (Uint(l), Int(r))=> Uint(l $o r as usize),
                  (Int(l), Int(r))=> Int(l $o r),
                  (Byte(l), Byte(r))=> Byte(l $o r),
                  (Byte(l), Int(r))=> Byte(l $o r as u8),
                  (Float(l), Float(r))=> Float(l $o r),
                  (Float(l), Int(r))=> Float(l $o r as f64),
                  _=> panic!("运算并赋值的左右类型不同 运行时({})", line)
                };
                *p = n;
              };
              self.modify_var(&id, f);
              return right;
            }
            self.err("只能为变量赋值。");
          }};
        }

        // 
        macro_rules! impl_unsigned_assign {
          ($op:tt) => {{
            if let Expr::Literal(Variant(id)) = &bin.left {
              let line = self.line;
              let f = |p: &mut Litr|{
                // 数字默认为Int，所以所有数字类型安置Int自动转换
                let n = match (left, right.clone()) {
                  (Uint(l), Uint(r))=> Uint(l $op r),
                  (Uint(l), Int(r))=> Uint(l $op r as usize),
                  (Byte(l), Byte(r))=> Byte(l $op r),
                  (Byte(l), Int(r))=> Byte(l $op r as u8),
                  _=> panic!("按位运算并赋值的左值有符号，或左右类型不同 运行时({})", line)
                };
                *p = n;
              };
              self.modify_var(&id, f);
              return right;
            }
            self.err("只能为变量赋值。");
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

          // 赋值
          b"=" => {
            if let Expr::Literal(Variant(id)) = &bin.left {
              self.modify_var(&id, |p|*p = right.clone());
              return right;
            }
            self.err("只能为变量赋值。");
          }
          b"+=" => impl_num_assign!(+),
          b"-=" => impl_num_assign!(-),
          b"*=" => impl_num_assign!(*),
          b"/=" => impl_num_assign!(/),
          b"%=" => impl_num_assign!(%),

          b"&=" => impl_unsigned_assign!(&),
          b"^=" => impl_unsigned_assign!(^),
          b"|=" => impl_unsigned_assign!(|),
          b"<<=" => impl_unsigned_assign!(<<),
          b">>=" => impl_unsigned_assign!(>>),

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
          _=> self.err(&format!("未知运算符'{}'", String::from_utf8_lossy(&bin.op)))
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

