use super::{Scanner, charts};
use crate::ast::{
  Litr, Expr, AccessDecl, BinDecl, CallDecl
};

pub fn expr(this:&Scanner)-> Expr {
  this.spaces();
  // 判断开头有无括号
  let left = if this.cur() == b'(' {
    this.expr_group()
  }else {
    this.literal()
  };
  this.expr_with_left(left)
}

pub fn with_left(this:&Scanner, left:Expr)-> Expr {
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
      if let Expr::Empty = second_last_expr {
        this.err("二元运算符未填写左值")
      }

      // 如果是模块或类的调用就不用Binary
      macro_rules! impl_access {($op:literal, $ty:ident)=>{{
        if last_op == $op.as_bytes() {
          if let Expr::Variant(left) = second_last_expr {
            if let Expr::Variant(right) = last_expr {
              expr_stack.push(Expr::$ty(Box::new(AccessDecl { left, right })));
              continue;
            }
            this.err(&format!("{}右侧需要一个标识符",$op))
          }
          this.err(&format!("{}左侧需要一个标识符",$op))
        }
      }}}
      impl_access!("-.",ModFuncAcc);
      impl_access!("-:",ModStruAcc);
      impl_access!("::",ImplAccess);

      expr_stack.push(Expr::Binary(Box::new(BinDecl { 
        left: second_last_expr, 
        right: last_expr, 
        op: last_op.to_vec()
      })));
    }

    // 运算符没有优先级则说明匹配结束
    if precedence == 0 {
      return expr_stack.pop().unwrap();
    }

    // 如果此运算符是括号就代表call
    if op == b"(" {
      let callee = expr_stack.pop().unwrap();
      let args = this.expr_group();
      expr_stack.push(Expr::Call(Box::new(CallDecl{
        args, targ:callee
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
  this.err(&format!("你需要为标识符 '{:?}' 后使用 ';' 结尾。", (&left)));
}

pub fn group(this:&Scanner)-> Expr {
  // 把左括号跳过去
  this.next();
  this.spaces();
  // 空括号作为空列表处理
  if this.cur() == b')' {
    this.next();
    return Expr::Literal(Litr::List(Box::new(Vec::new())));
  }

  let expr = this.expr();
  this.spaces();
  if this.i() >= this.src.len() || this.cur() != b')' {
    this.err("未闭合的右括号')'。");
  }
  this.next();
  expr
}