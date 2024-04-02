use super::{Scanner, charts};
use crate::primitive::litr::{
  KsType, Litr, LocalFuncRaw
};
use crate::intern::{intern, Interned};

/// 可以出现在任何右值的，expression表达式
#[derive(Debug, Clone)]
pub enum Expr {
  Empty,
  /// 字面量
  Literal(Litr),
  /// 变量
  Variant(Interned),
  /// self
  Kself,

  /// 未绑定作用域的本地函数
  LocalDecl (LocalFuncRaw),

  /// -.运算符 module-.func
  ModFuncAcc(Interned, Interned),
  /// -:运算符 module-:Class
  ModClsAcc (Interned, Interned),
  /// .运算符 a.b
  Property  (Box<Expr>, Interned),
  /// ::运算符 Class::static_method
  ImplAccess(Box<Expr>, Interned),

  /// 调用函数 x()
  Call {
    args: Vec<Expr>,
    targ: Box<Expr>
  },

  /// 调用方法 x.method()
  CallMethod {
    args: Vec<Expr>,
    targ: Box<Expr>,
    name: Interned
  },

  /// 索引表达式
  Index{
    left: Box<Expr>,
    i: Box<Expr>
  },

  /// 创建实例
  NewInst{
    cls: Box<Expr>,
    val: Vec<(Interned,Expr)>
  },

  /// 列表表达式
  List(Vec<Expr>),
  /// 对象表达式
  Obj(Vec<(Interned,Expr)>),

  /// 一元运算 ! -
  Unary{
    right: Box<Expr>,
    op: u8
  },

  /// 二元运算
  Binary{
    left: Box<Expr>,
    right: Box<Expr>,
    op: Box<[u8]>
  },

  /// is表达式 a is ClassA
  Is {
    left: Box<Expr>,
    right: Box<Expr>
  }
}

/// 使用|>时会将左侧表达式暂存此处, 使用|%|时被取走
pub static mut ON_PIPE:Option<Expr> = None;

impl Scanner<'_> {
  /// 从self.i直接开始解析一段表达式
  pub fn expr(&self)-> Expr {
    self.spaces();
    let unary = self.operator_unary();
    self.spaces();
    // 判断开头有无括号
    let left = if self.cur() == b'(' {
      self.expr_group()
    }else {
      self.literal()
    };
    self.expr_with_left(left, unary)
  }

  /// 匹配一段表达式，传入二元表达式左边部分和一元运算符
  pub fn expr_with_left(&self, left:Expr, mut unary:Vec<u8>)-> Expr {
    use charts::prec;

    let mut expr_stack = vec![left];
    let mut op_stack = Vec::<&[u8]>::new();
  
    let len = self.src.len();
    loop {
      // 向后检索二元运算符
      self.spaces();
      let op = self.operator();
      let precedence = prec(op);

      // 在新运算符加入之前，根据二元运算符优先级执行合并
      while let Some(last_op) = op_stack.pop() {
        let last_op_prec = prec(last_op);
        // 只有在这次运算符优先级无效 或 小于等于上个运算符优先级才能进行合并
        if precedence > last_op_prec && precedence != 0 {
          op_stack.push(last_op);
          break;
        }

        let right = expr_stack.pop().unwrap();
        let left = expr_stack.pop().unwrap();

        // 对于模块访问左右都必须是标识符
        macro_rules! impl_access {($op:literal, $ty:ident)=>{{
          if last_op == $op {
            if let Expr::Variant(left) = left {
              if let Expr::Variant(right) = right {
                expr_stack.push(Expr::$ty(left, right));
                continue;
              }
              panic!("{}右侧需要一个标识符",String::from_utf8_lossy($op))
            }
            panic!("{}左侧需要一个标识符",String::from_utf8_lossy($op))
          }
        }}}
        impl_access!(b"-.",ModFuncAcc);
        impl_access!(b"-:",ModClsAcc);

        // ::表达式
        if last_op == b"::" {
          match right {
            Expr::Variant(id)=> 
              expr_stack.push(Expr::ImplAccess(Box::new(left), id)),
            Expr::Obj(o)=> 
              expr_stack.push(Expr::NewInst { cls: Box::new(left), val: o }),
            _=> panic!("::右侧只能是标识符或对象")
          }
          continue;
        }

        // is表达式
        if last_op == b"is" {
          expr_stack.push(Expr::Is {
            left: Box::new(left),
            right: Box::new(right)
          });
          continue;
        }

        expr_stack.push(Expr::Binary{ 
          left: Box::new(left), 
          right: Box::new(right), 
          op: last_op.into()
        });
      }

      // 如果没匹配到运算符就说明匹配结束
      if op.len() == 0 {
        assert_eq!(expr_stack.len(), 1);
        return expr_stack.pop().unwrap();
      }

      // 对二元运算符的各种情况做处理
      match op {
        // 如果用户想用返回语句就直接以此分界
        b":"=> {
          unsafe {(*self.i) -= 1;}
          return expr_stack.pop().unwrap();
        }

        // 如果此运算符是括号就代表call
        b"("=> {
          self.next();
          self.spaces();
          let targ = Box::new(expr_stack.pop().unwrap());
          let mut args = parse_input_args(self);
          expr_stack.push(Expr::Call { args, targ });
          continue;
        }

        // 如果是.就说明是属性或者调用方法
        b"."=> {
          let left = Box::new(expr_stack.pop().unwrap());
          let name = match self.ident() {
            Some(n)=> intern(n),
            None=> panic!("'.'右边需要属性名")
          };
          self.spaces();
          // 属性后直接使用括号就是调用方法
          if self.cur() == b'(' {
            self.next();
            let args = parse_input_args(self);
            expr_stack.push(Expr::CallMethod { args, targ: left, name });
          }else {
            expr_stack.push(Expr::Property(left, name));
          }
          continue;
        }

        // 如果此运算符是方括号就代表index
        b"["=> {
          self.next();
          self.spaces();
          let left = Box::new(expr_stack.pop().unwrap());
          let i = Box::new(self.expr());
          if self.i() >= self.src.len() || self.cur() != b']' {
            panic!("未闭合的右括号']'。");
          }
          self.next();
          expr_stack.push(Expr::Index{
            left, i
          });
          continue;
        }

        // 管道运算符
        // 该运算符不是真的运算符, 只是一个语法糖
        b"|>"=> {
          unsafe{ ON_PIPE = Some(expr_stack.pop().unwrap()); }
          // |>的优先级是1,最低的,保证了expr_stack已经被合并成一个Expr了
          // 此时该函数上下文已经没用了, 可以直接再开始一次expr
          return self.expr();
        }
        _=>()
      }

      // 将新运算符和它右边的值推进栈
      self.spaces();

      // 看看右侧值前有没有一元运算符
      let mut una = self.operator_unary();
      unary.append(&mut una);

      // 优先级够的话,合并一元运算符
      if precedence < charts::PREC_UNARY && unary.len() > 0 {
        let mut right = expr_stack.pop().unwrap();
        while let Some(op) = unary.pop() {
          right = Expr::Unary { right:Box::new(right), op }
        }
        expr_stack.push(right);
      }

      // 在此之前判断有没有括号来提升优先级
      let right = if self.cur() == b'(' {
        self.expr_group()
      }else {
        self.literal()
      };
      expr_stack.push(right);

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
      panic!("未闭合的右括号')'。");
    }
    self.next();
    expr
  }

  /// 检索一段 二元操作符
  fn operator(&self)-> &[u8] {
    let len = self.src.len();
    if self.i()>=len {return b"";}
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

      // |开头的运算符只允许|,|>和||,防止和闭包与|%|混淆
      b'|'=> {
        self.next();
        if self.i()>=len {
          return b"|";
        }
        match self.cur() {
          b'|'=> {
            self.next();
            return b"||";
          }
          b'>'=> {
            self.next();
            return b"|>";
          }
          _=> return b"|"
        }
      }
      _=>()
    }

    let mut i = self.i();
    while i < len {
      let cur = self.src[i];
      match cur {
        b'!'|b'%'|b'&'|b'*'|b'+'|b'-'|b'.'|b'/'|b'<'|b'>'|b'='|b'^'|b':'=> {
          i += 1;
        }
        _=> break
      }
    }

    let op = &self.src[self.i()..i];
    self.set_i(i);
    op
  }

  /// 检查有没有一元运算符
  fn operator_unary(&self)-> Vec<u8> {
    let mut v = Vec::new();
    loop {
      let cur = self.cur();
      match cur {
        b'!' | b'-'=> {
          self.next();
          v.push(cur);
          self.spaces();
        }
        _=> break
      }
    }
    v
  }
}

/// 解析传入参数
fn parse_input_args(this:&Scanner)-> Vec<Expr> {
  let mut args = Vec::new();
  // 如果直接遇到右括号则代表无参数传入
  if this.cur() == b')' {
    this.next();
    return args;
  }

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
    panic!("未闭合的右括号')'。");
  }
  this.next();
  args
}