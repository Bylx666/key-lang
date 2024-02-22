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
  LocalDecl (LocalFuncRaw),

  // -.运算符
  ModFuncAcc(Interned, Interned),
  // -:运算符
  ModClsAcc (Interned, Interned),
  // .运算符
  Property  (Box<Expr>, Interned),
  // ::运算符
  ImplAccess(Box<Expr>, Interned),
  // 调用函数
  Call{
    args: Vec<Expr>,
    targ: Box<Expr>
  },
  // 创建实例
  NewInst{
    cls: Interned,
    val: Vec<(Interned,Expr)>
  },

  // 列表表达式
  List(Vec<Expr>),
  // 对象表达式
  Obj(Vec<(Interned,Expr)>),

  // 一元运算 ! -
  Unary{
    right: Box<Expr>,
    op: u8
  },

  // 二元运算
  Binary{
    left: Box<Expr>,
    right: Box<Expr>,
    op: Box<[u8]>
  },
}


impl Scanner<'_> {
  /// 从self.i直接开始解析一段表达式
  pub fn expr(&self)-> Expr {
    self.spaces();
    // 判断开头有无括号
    let left = if self.cur() == b'(' {
      self.expr_group()
    }else {
      self.literal()
    };
    self.expr_with_left(left)
  }

  /// 匹配一段表达式，传入二元表达式左边部分
  pub fn expr_with_left(&self, left:Expr)-> Expr {
    use charts::prec;
  
    let mut expr_stack = vec![left];
    let mut op_stack = Vec::<&[u8]>::new();
  
    let len = self.src.len();
    loop {
      // 向后检索二元运算符
      self.spaces();
      let op = self.operator();
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
                expr_stack.push(Expr::$ty(left, right));
                continue;
              }
              self.err(&format!("{}右侧需要一个标识符",String::from_utf8_lossy($op)))
            }
            self.err(&format!("{}左侧需要一个标识符",String::from_utf8_lossy($op)))
          }
        }}}
        impl_access!(b"-.",ModFuncAcc);
        impl_access!(b"-:",ModClsAcc);
  
        // .和::都是左边是表达式，右边是标识符
        macro_rules! impl_prop {($op:literal, $ty:ident) => {
          if last_op == $op {
            if let Expr::Variant(right) = last_expr {
              expr_stack.push(Expr::$ty(Box::new(second_last_expr), right ));
              continue;
            }
            self.err(&format!("{}右侧需要一个标识符",String::from_utf8_lossy($op)))
          }
        }}
        impl_prop!(b".", Property);
        impl_prop!(b"::", ImplAccess);
  
        expr_stack.push(Expr::Binary{ 
          left: Box::new(second_last_expr), 
          right: Box::new(last_expr), 
          op: last_op.into()
        });
      }
  
      // 如果没匹配到运算符就说明匹配结束
      if op.len() == 0 {
        return expr_stack.pop().unwrap();
      }
  
      // 如果此运算符是括号就代表call
      if op == b"(" {
        let targ = Box::new(expr_stack.pop().unwrap());
        self.next();
        self.spaces();
        let mut args = Vec::new();
        loop {
          let e = self.expr();
          // 调用参数留空就当作uninit
          args.push(if let Expr::Empty = e {
            Expr::Literal(Litr::Uninit)
          }else {e});
          self.spaces();
          if self.cur() != b',' {
            break;
          }
          self.next();
        }
        if self.i() >= self.src.len() || self.cur() != b')' {
          self.err("未闭合的右括号')'。");
        }
        self.next();
        expr_stack.push(Expr::Call{
          args, targ
        });
        continue;
      };
  
      // 将新运算符和它右边的值推进栈
      self.spaces();
      // 在此之前判断有没有括号来提升优先级
      if self.cur() == b'(' {
        let group = self.expr_group();
        expr_stack.push(group);
      }else {
        let litr = self.literal();
        expr_stack.push(litr);
      }
      op_stack.push(op);
  
    }
  }
  
  /// 匹配带括号的表达式(提升优先级和函数调用)
  pub fn expr_group(&self)-> Expr {
    // 把左括号跳过去
    self.next();
    self.spaces();
    // 空括号作为空列表处理
    if self.cur() == b')' {
      self.next();
      return Expr::Literal(Litr::Uninit);
    }
  
    let expr = self.expr();
    self.spaces();
    if self.i() >= self.src.len() || self.cur() != b')' {
      self.err("未闭合的右括号')'。");
    }
    self.next();
    expr
  }
}
