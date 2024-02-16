use super::{Scanner, charts};
use super::literal::{
  Litr, LocalFuncRaw
};
use crate::intern::Interned;

/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Empty,
  // 字面量
  Literal(Litr),
  // 变量
  Variant(Interned),
  // self
  Kself,

  // 未绑定作用域的本地函数
  LocalDecl (Box<LocalFuncRaw>),

  // .运算符
  Property  (Box<(Expr, Interned)>),
  // -.运算符
  ModFuncAcc(Box<(Interned, Interned)>),
  // -:运算符
  ModClsAcc (Box<(Interned, Interned)>),
  // ::运算符
  ImplAccess(Box<(Expr, Interned)>),
  // 调用函数
  Call      (Box<CallDecl>),
  // 创建实例
  NewInst   (Box<NewDecl>),

  // 列表表达式
  List      (Box<Vec<Expr>>),
  // 对象表达式
  Obj       (Box<ObjDecl>),

  // 一元运算 ! -
  Unary     (Box<UnaryDecl>),
  // 二元运算
  Binary    (Box<BinDecl>),
}

// V 注释见Expr V

#[derive(Debug, Clone)]
pub struct BinDecl {
  pub left: Expr,
  pub right: Expr,
  pub op: Box<[u8]>
}

#[derive(Debug, Clone)]
pub struct UnaryDecl {
  pub right: Expr,
  pub op: u8
}


#[derive(Debug, Clone)]
pub struct CallDecl {
  pub args: Vec<Expr>,
  pub targ: Expr
}

#[derive(Debug, Clone)]
pub struct NewDecl {
  pub cls: Interned,
  pub val: ObjDecl
}

#[derive(Debug, Clone)]
pub struct ObjDecl (
  pub Vec<(Interned,Expr)>
);


pub(super) fn expr(this:&Scanner)-> Expr {
  this.spaces();
  // 判断开头有无括号
  let left = if this.cur() == b'(' {
    this.expr_group()
  }else {
    this.literal()
  };
  this.expr_with_left(left)
}

pub(super) fn with_left(this:&Scanner, left:Expr)-> Expr {
  use charts::prec;

  let mut expr_stack = vec![left];
  let mut op_stack = Vec::<&[u8]>::new();

  let len = this.src.len();
  loop {
    // 向后检索二元运算符
    this.spaces();
    let op = this.operator();
    let precedence = prec(op);

    // 在新运算符加入之前，根据运算符优先级执行合并
    while let Some(last_op) = op_stack.pop() {
      let last_op_prec = prec(last_op);
      // 只有在这次运算符优先级无效 或 小于等于上个运算符优先级才能进行合并
      if precedence > last_op_prec && precedence != 0 {
        op_stack.push(last_op);
        break;
      }

      let last_expr = expr_stack.pop().unwrap();
      let second_last_expr = expr_stack.pop().unwrap();

      // 如果是模块或类的调用就不用Binary
      macro_rules! impl_access {($op:literal, $ty:ident)=>{{
        if last_op == $op {
          if let Expr::Variant(left) = second_last_expr {
            if let Expr::Variant(right) = last_expr {
              expr_stack.push(Expr::$ty(Box::new((left, right))));
              continue;
            }
            this.err(&format!("{}右侧需要一个标识符",String::from_utf8_lossy($op)))
          }
          this.err(&format!("{}左侧需要一个标识符",String::from_utf8_lossy($op)))
        }
      }}}
      impl_access!(b"-.",ModFuncAcc);
      impl_access!(b"-:",ModClsAcc);

      // .和::都是左边是表达式，右边是标识符
      macro_rules! impl_prop {($op:literal, $ty:ident) => {
        if last_op == $op {
          if let Expr::Variant(right) = last_expr {
            expr_stack.push(Expr::$ty(Box::new(( second_last_expr, right ))));
            continue;
          }
          this.err(&format!("{}右侧需要一个标识符",String::from_utf8_lossy($op)))
        }
      }}
      impl_prop!(b".", Property);
      impl_prop!(b"::", ImplAccess);

      expr_stack.push(Expr::Binary(Box::new(BinDecl { 
        left: second_last_expr, 
        right: last_expr, 
        op: last_op.into()
      })));
    }

    // 如果没匹配到运算符就说明匹配结束
    if op.len() == 0 {
      return expr_stack.pop().unwrap();
    }

    // 如果此运算符是括号就代表call
    if op == b"(" {
      let targ = expr_stack.pop().unwrap();
      this.next();
      this.spaces();
      let mut args = Vec::new();
      loop {
        let e = this.expr();
        // 调用参数留空就当作uninit
        args.push(if let Expr::Empty = e {
          Expr::Literal(Litr::Uninit)
        }else {e});
        this.spaces();
        if this.cur() != b',' {
          break;
        }
        this.next();
      }
      if this.i() >= this.src.len() || this.cur() != b')' {
        this.err("未闭合的右括号')'。");
      }
      this.next();
      expr_stack.push(Expr::Call(Box::new(CallDecl{
        args, targ
      })));
      continue;
    };

    // 将新运算符和它右边的值推进栈
    this.spaces();
    // 在此之前判断有没有括号来提升优先级
    if this.cur() == b'(' {
      let group = this.expr_group();
      expr_stack.push(group);
    }else {
      let litr = this.literal();
      expr_stack.push(litr);
    }
    op_stack.push(op);

  }
}

pub(super) fn group(this:&Scanner)-> Expr {
  // 把左括号跳过去
  this.next();
  this.spaces();
  // 空括号作为空列表处理
  if this.cur() == b')' {
    this.next();
    return Expr::Literal(Litr::Uninit);
  }

  let expr = this.expr();
  this.spaces();
  if this.i() >= this.src.len() || this.cur() != b')' {
    this.err("未闭合的右括号')'。");
  }
  this.next();
  expr
}