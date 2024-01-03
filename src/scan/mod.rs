//! 将源码扫描为 AST的过程

use std::collections::HashMap;

use crate::ast::{
  Expr, 
  Stmt,
  Litr, 
  KsType,
  Ident, 
  Executable, 
  Statements, 
  BinCalc, 
  KsCall, 
  KsAssign,
};

mod prec;
use prec::prec;


/// 将字符整理为ast
pub fn scan(src: Vec<u8>)-> Statements {
  let mut scanner = Scanner {src, i:0, sttms:Statements::default()};
  scanner.sttms.line += 1;
  scanner.scan()
}
struct Scanner {
  src: Vec<u8>,
  i: usize,
  sttms: Statements,
}
impl Scanner {
  /// 启动扫描并返回Ast
  fn scan(mut self)-> Statements {
    let mut stats = Vec::<Stmt>::new();
    let len = self.src.len();
    while self.i < len {
      self.statem();
    }
    self.sttms
  }

  fn push(&mut self, s:Stmt) {
    self.sttms.exec.push((self.sttms.line, s));
  }

  /// 匹配一个语句
  fn statem(&mut self) {
    self.spaces();
    if self.i >= self.src.len() {
      self.i += 1; // 打破scan函数的while
      return;
    }

    // 分号开头即为空语句
    let first_char = self.cur();
    if first_char == 0x3B {
      self.i += 1;
      return;
    }

    let ident = self.ident();
    if let Some(id) = ident {
      match &*id {
        // 如果是关键词，就会让对应函数处理关键词之后的信息
        b"let"=> self.letting(),
        _=> {
          let id = Expr::Literal(Litr::Variant(Box::new(id)));
          let expr = Box::new(self.expr_with_left(id));
          self.push(Stmt::Expression(expr))
        }
      }
    }else {
      let expr = Box::new(self.expr());
      self.push(Stmt::Expression(expr));
    }
  }


  // ==== Expr Start ==== //
  /// 从self.i直接开始解析一段表达式
  fn expr(&mut self)-> Expr {
    if self.cur() == 0x28 {
      return self.expr_group();
    }
    let left = Expr::Literal(self.literal());
    self.expr_with_left(left)
  }

  /// 匹配一段表达式，传入二元表达式左边部分
  fn expr_with_left(&mut self, left:Expr)-> Expr {
    let mut expr_stack = vec![self.maybe_index_call(left)];
    let mut op_stack = Vec::<u8>::new();

    let len = self.src.len();
    loop {
      // 向后检索二元运算符
      self.spaces();
      let op = self.cur();
      let precedence = prec(op);

      // 在新运算符加入之前，根据运算符优先级执行合并
      while let Some(last_op) = op_stack.pop() {
        let last_op_prec = prec(last_op);
        // 只有在这次运算符优先级无效 或 小于等于上个运算符优先级才能进行合并
        if precedence > last_op_prec && precedence != 0 {
          op_stack.push(last_op);
          break;
        }

        let last_expr = unsafe{ expr_stack.pop().unwrap_unchecked() };
        let second_last_expr = unsafe{ expr_stack.pop().unwrap_unchecked() };
        expr_stack.push(Expr::Binary(Box::new(BinCalc { 
          left: second_last_expr, 
          right: last_expr, 
          sym: last_op
        })));
      }

      // 运算符没有优先级则说明匹配结束
      if precedence == 0 {
        assert_eq!(expr_stack.len(), 1);
        return expr_stack.pop().unwrap();
      }
      // 运算符有优先级就把i向后拨一位跳过这个运算符
      self.i += 1;

      // 将新运算符和它右边的值推进栈
      let right = Expr::Literal(self.literal());
      let right = self.maybe_index_call(right);
      expr_stack.push(right);
      op_stack.push(op);

    }
    self.err(&format!("你需要为标识符 '{:?}' 后使用 ';' 结尾。", (&left)));
  }

  /// 匹配带括号的表达式(提升优先级和函数调用)
  /// 
  /// 参数这东西不管你传了几个，到最后都是一个Expr，神奇吧
  fn expr_group(&mut self)-> Expr {
    // 把左括号跳过去
    self.i += 1;

    let expr = self.expr();
    self.spaces();
    if self.i >= self.src.len() || self.cur() != 0x29 {
      self.err("未闭合的右括号')'。");
    }
    self.i += 1;
    expr
  }

  /// 看Expr后面有没有call或index
  #[inline]
  fn maybe_index_call(&mut self, e:Expr)-> Expr {
    if self.cur() == 0x28 {
      let args = self.expr_group();
      return  Expr::Call(Box::new(KsCall{
        args, targ:e
      }))
    }
    e
  }
  // ==== Expr End ==== //


  /// 匹配标识符(如果匹配不到则返回的vec.len()为0)
  fn ident(&mut self)-> Option<Ident> {
    self.spaces();

    let mut i = self.i;
    let len = self.src.len();

    // 判断首字是否为数字
    let first = self.src[i];
    if first>=0x30 && first<=0x39 {return None;}

    loop {
      if i >= len {panic!();}
      let s = self.src[i];
      match s {
        0x5F | 0x24 | 0x7E | 0x40 | // _ $ ~ @
        0x41..=0x5A | 0x61..=0x7A | // 大写 小写
        0x30..=0x39 => {            // 数字
          i += 1;
        },
        _=> {
          if self.i == i {return None;}
          let ident = self.src[self.i..i].to_vec();
          self.i = i;
          return Some(ident);
        }
      }
    }
  }


  /// 解析一段字面量
  fn literal(&mut self)-> Litr {
    self.spaces();

    let first = self.cur();
    let len = self.src.len();

    let mut i = self.i;
    match first {
      // 解析字符字面量
      0x22 => {
        while self.src[i] != 0x22 {
          i += 1;
          if i >= len {self.err("未找到字符串结尾的'\"'。")}
        }
        let s = self.src[(self.i+1)..i].to_vec();
        self.i = i + 1;
        Litr::Str(Box::new(s))
      }
      // 解析数字字面量
      0x30..=0x39 => {
        loop {
          i += 1;
          if i >= len {self.err("数字后请使用';'结束。")}
          if !(0x30..=0x39).contains(&self.src[i]) {break;}
        }
        let str = unsafe{
          String::from_utf8_unchecked(self.src[self.i..i].to_vec())
        };
        let res:Result<isize, _> = str.parse();
        if let Ok(n) = res {
          self.i = i;
          return Litr::Int(n);
        }else {
          self.err(&format!("数字超出范围:{}。", str))
        }
      },
      _=> {
        let id_res = self.ident();
        if let Some(id) = id_res {
          Litr::Variant(Box::new(id))
        }else {
          self.err("无法解析字面量。");
        }
      }
    }
  }


  /// 跳过一段空格和换行符
  fn spaces(&mut self) {
    let len = self.src.len();
    while self.i < len {
      let c = self.cur();
      if c == 0x0A {
        self.sttms.line += 1;
      }
      match c {
        0x20 | 0x0D | 0x0A => {
          self.i += 1;
        },
        _=> {
          break;
        }
      }
    }
  }


  /// 解析let关键词
  fn letting(&mut self) {
    let id = self.ident();
    if id.is_none() {self.err("let后需要标识符。")}
    let id = id.unwrap();

    // 暂时不做关键词检查，可以略微提升性能

    // 检查标识符后的符号
    self.spaces();
    let sym = self.cur();
    match sym {
      0x3D => { // =
        self.i += 1;
        let val = self.expr();
        self.push(Stmt::Let(Box::new(KsAssign {
          id, val
        })));
      }
      0x3B => { // ;
        self.i += 1;
        self.push(Stmt::Let(Box::new(KsAssign {
          id, val:Expr::Literal(Litr::Uninit)
        })));
      }
      _ => self.err(&format!("需要';'或'='，但你输入了{}", sym))
    }
  }


  /// 获取当前u8
  #[inline]
  fn cur(&self)-> u8 {
    unsafe { *self.src.get_unchecked(self.i) }
  }

  /// 报错
  fn err(&self, s:&str)-> ! {
    panic!("{} 解析错误({})",s,self.sttms.line)
  }
}
