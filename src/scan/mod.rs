//! 将源码扫描为 AST的过程

use std::collections::HashMap;
use std::f32::consts::E;

use crate::ast::*;
use crate::intern::{
  intern,
  Interned
};
use crate::allocated::leak;

mod charts;

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

  /// 跳过一段空格和换行符
  fn spaces(&self) {
    let len = self.src.len();
    while self.i() < len {
      let c = self.cur();
      if c == 0x0A {
        unsafe{*self.line += 1;}
      }
      match c {
        0x20 | 0x0D | 0x0A => {
          self.next();
        },
        _=> {
          break;
        }
      }
    }
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
}



/// 语句方法
impl Scanner<'_> {
  /// 匹配一个语句
  fn stmt(&self)-> Stmt {
    self.spaces();
    if self.i() >= self.src.len() {
      self.next(); // 打破scan函数的while
      return Stmt::Empty;
    }

    let first = self.cur();
    match first {
      // 分号开头即为空语句
      b';' => {
        self.next();
        return Stmt::Empty;
      }
      // 块语句
      b'{' => {
        let mut stmts = Statements::default();
        let len = self.src.len();
        self.next();
        loop {
          if self.i() >= len {
            self.err("未闭合的块大括号");
          }
          self.spaces();
          if self.cur() == b'}' {
            self.next();
            return Stmt::Block(Box::new(stmts));
          }
          
          let s = self.stmt();
          if let Stmt::Empty = s {
            continue;
          }
          stmts.0.push((self.line(), s));
        }
      }
      // 返回语句语法糖
      b':' => {
        self.next();
        return self.returning();
      }
      _=>{}
    }

    let ident = self.ident();
    if let Some(id) = ident {
      match id {
        // 如果是关键词，就会让对应函数处理关键词之后的信息
        b"let"=> self.letting(),
        b"extern"=> self.externing(),
        b"return"=> self.returning(),
        b"struct"=> self.structing(),
        b"mod"=> self.moding(),
        _=> {
          let interned = intern(id);
          let id = Expr::Literal(Litr::Variant(interned));
          let expr = Box::new(self.expr_with_left(id));
          return Stmt::Expression(expr);
        }
      }
    }else {
      let expr = self.expr();
      if let Expr::Empty = expr {
        self.err(&format!("请输入一行正确的语句，'{}'并不合法", String::from_utf8_lossy(&[self.cur()])))
      }
      return Stmt::Expression(Box::new(expr));
    }
  }

  /// 解析let关键词
  fn letting(&self)-> Stmt {
    self.spaces();
    let id = self.ident().unwrap_or_else(||self.err("let后需要标识符"));
    let id = intern(id);

    // 检查标识符后的符号
    self.spaces();
    let sym = self.cur();
    match sym {
      b'=' => {
        self.next();
        let val = self.expr();
        if let Expr::Empty = val {
          self.err("无法为空气赋值")
        }
        return Stmt::Let(Box::new(AssignDef {
          id, val
        }));
      }
      b'(' => {
        self.next();
        let args = self.arguments();
        if self.cur() != b')' {
          self.err("函数声明右括号缺失");
        }
        self.next();

        let stmt = self.stmt();
        let mut exec = if let Stmt::Block(b) = stmt {
          *b
        }else {
          Statements(vec![(self.line(), stmt)])
        };
        let scope = std::ptr::null_mut();
        let func = Box::new(Executable::Local(Box::new(LocalFunc { argdecl: args, exec, scope })));
        return Stmt::Let(Box::new(AssignDef { 
          id, 
          val: Expr::Literal(Litr::Func(func))
        }));
      }
      _ => {
        return Stmt::Let(Box::new(AssignDef {
          id, val:Expr::Literal(Litr::Uninit)
        }));
      }
    }
  }

  /// extern关键词
  /// 
  /// 会固定返回空语句
  fn externing(&self)-> Stmt {
    use crate::c::Clib;

    // 截取路径
    self.spaces();
    let mut i = self.i();
    let len = self.src.len();
    while self.src[i] != b'>' {
      if i >= len {
        self.err("extern后需要 > 符号");
      }
      i += 1;
    }

    let path = &self.src[self.i()..i];
    let lib = Clib::load(path).unwrap_or_else(|e|self.err(&e));
    self.set_i(i + 1);
    self.spaces();

    /// 解析并推走一个函数声明
    macro_rules! parse_decl {($id:ident) => {{
      let sym:&[u8];
      // 别名解析
      if self.cur() == b':' {
        self.next();
        self.spaces();
        if let Some(i) = self.ident() {
          sym = i;
        }else {
          self.err(":后需要别名")
        };
      }else {
        sym = $id;
      }

      // 解析小括号包裹的参数声明
      if self.cur() != b'(' {
        self.err("extern函数后应有括号");
      }
      self.next();
      let argdecl = self.arguments();
      self.spaces();
      if self.cur() != b')' {
        self.err("extern函数声明右括号缺失");
      }
      self.next();
      
      if self.cur() == b';' {
        self.next();
      }

      // 将函数名(id)和指针(ptr)作为赋值语句推到语句列表里
      let ptr = lib.get(sym).unwrap_or_else(||self.err(
        &format!("动态库'{}'中不存在'{}'函数", 
        String::from_utf8_lossy(path), 
        String::from_utf8_lossy(sym))));
      self.push(Stmt::Let(Box::new(AssignDef { 
        id:intern($id), 
        val: Expr::Literal(Litr::Func(Box::new(Executable::Extern(Box::new(ExternFunc { 
          argdecl, 
          ptr
        })))))
      })));
    }}}


    // 大括号语法
    if self.cur() == b'{' {
      self.next();
      self.spaces();
      while let Some(id) = self.ident() {
        parse_decl!(id);
        self.spaces();
      }
      self.spaces();
      if self.cur() != b'}' {
        self.err("extern大括号未闭合")
      }
      self.next();
      return Stmt::Empty;
      
    }else {
      // 省略大括号语法
      let id = self.ident().unwrap_or_else(||self.err("extern后应有函数名"));
      parse_decl!(id);
    }

    Stmt::Empty
  }

  /// 解析返回值
  fn returning(&self)-> Stmt {
    self.spaces();
    let expr = self.expr();
    if let Expr::Empty = expr {
      Stmt::Return(Box::new(Expr::Literal(Litr::Uninit)))
    }else {
      Stmt::Return(Box::new(expr))
    }
  }

  /// 解析结构体声明
  fn structing(&self)-> Stmt {
    self.spaces();
    let id = self.ident().unwrap_or_else(||self.err("struct后需要标识符"));
    self.spaces();
    if self.cur() != b'{' {
      self.err("struct需要大括号");
    }
    self.next();

    let def = self.arguments();
    
    todo!();
    self.spaces();
    let mut layout = Vec::<(Interned,)>::new();
    while let Some(n) = self.ident() {
      let arg = intern(n);
      if self.cur() != b':' {
        self.err("必须使用类型声明");
      }
      self.next();
      let typ = self.ident().unwrap_or_else(||self.err("类型声明不能为空"));
      let typ = {
        use StructElemType::*;
        match typ {
          b"Uint8"=> Uint8,
          b"Uint16"=> Uint16,
          b"Uint32"=> Uint32,
          b"Uint"=> Uint,
          _=> Structp
        }
      };

      self.spaces();
      if self.cur() == b',' {
        self.next();
      }
      self.spaces();
    }

    self.spaces();
    if self.cur() != b'}' {
      self.err("struct大括号未闭合");
    }
    self.next();
    Stmt::Struct(Box::new(StructDef (def)))
  }

  /// 解析模块声明
  fn moding(&self)-> Stmt {
    // 截取路径
    self.spaces();
    let mut i = self.i();
    let len = self.src.len();
    let mut dot = 0;
    loop {
      let cur = self.src[i];
      if cur == b'>' {
        break;
      }
      if cur == b'.' {
        dot = i;
      }
      if i >= len {
        self.err("extern后需要 > 符号");
      }
      i += 1;
    }
  
    let path = &self.src[self.i()..i];
    self.set_i(i + 1);

    self.spaces();
    let name = self.ident().unwrap_or_else(||self.err("需要为模块命名"));
    self.spaces();

    if dot == 0 {
      self.err("未知模块类型")
    }
    let suffix = &self.src[dot..i];
    match suffix {
      b".ksm"|b".dll"=> {
        let module = crate::module::parse(name, path).unwrap_or_else(|e|
          self.err(&format!("模块解析失败:{}\n  {}",e,String::from_utf8_lossy(path))));
        Stmt::Mod(Box::new(module))
      }
      b".ks"=> {
        // let file = std::fs::read(path).unwrap_or_else(|e|self.err(&format!(
        //   "无法找到模块'{}'", path
        // )));
        // scan(file)
        Stmt::Empty
      }
      _ => self.err("未知模块类型")
    }
  }
}



/// 表达式方法
impl Scanner<'_> {
  /// 从self.i直接开始解析一段表达式
  fn expr(&self)-> Expr {
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
  fn expr_with_left(&self, left:Expr)-> Expr {
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
        if let Expr::Empty = second_last_expr {
          self.err("二元运算符未填写左值")
        }

        // 如果是模块或类的调用就不用Binary
        macro_rules! impl_access {($op:literal, $ty:ident)=>{{
          if last_op == $op {
            if let Expr::Literal(Litr::Variant(left)) = second_last_expr {
              if let Expr::Literal(Litr::Variant(right)) = last_expr {
                expr_stack.push(Expr::$ty(Box::new(AccessDecl { left, right })));
                continue;
              }
              self.err("运算符右侧需要一个标识符")
            }
            self.err("运算符左侧需要一个标识符")
          }
        }}}
        impl_access!(b"-.",ModFuncAcc);
        impl_access!(b"-:",ModStruAcc);
        impl_access!(b"::",ImplAccess);

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
        let args = self.expr_group();
        expr_stack.push(Expr::Call(Box::new(CallDecl{
          args, targ:callee
        })));
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
    self.err(&format!("你需要为标识符 '{:?}' 后使用 ';' 结尾。", (&left)));
  }

  /// 匹配带括号的表达式(提升优先级和函数调用)
  /// 
  /// 参数这东西不管你传了几个，到最后都是一个Expr，神奇吧
  fn expr_group(&self)-> Expr {
    // 把左括号跳过去
    self.next();
    self.spaces();
    // 空括号作为空列表处理
    if self.cur() == b')' {
      self.next();
      return Expr::Literal(Litr::Array(Box::new(Vec::new())));
    }

    let expr = self.expr();
    self.spaces();
    if self.i() >= self.src.len() || self.cur() != b')' {
      self.err("未闭合的右括号')'。");
    }
    self.next();
    expr
  }

  /// 看Expr后面有没有call或index
  #[inline]
  fn maybe_index_call(&self, e:Expr)-> Expr {
    if self.cur() == b'(' {
      let args = self.expr_group();
      return  Expr::Call(Box::new(CallDecl{
        args, targ:e
      }))
    }
    e
  }


  /// 检索一段 二元操作符
  fn operator(&self)-> &[u8] {
    let mut i = self.i();
    let len = self.src.len();
    while i < len {
      let cur = self.src[i];
      match cur {
        b'%'|b'&'|b'*'|b'+'|b','|b'-'|b'.'|b'/'|b'<'|b'>'|b'='|b'^'|b'|'=> {
          i += 1;
        }
        b'(' => return b"(",
        b'[' => return b"[",
        _=> break
      }
    }

    let op = &self.src[self.i()..i];
    self.set_i(i);
    return op;
  }
}



/// 字面量方法
impl Scanner<'_> {
  /// 解析一段字面量
  /// 
  /// 同时解析一元运算符
  fn literal(&self)-> Expr {
    let first = self.cur();
    let len = self.src.len();
    let mut i = self.i();

    macro_rules! match_unary {($o:expr) => {{
      self.next();
      let right = self.literal();
      return Expr::Unary(Box::new(UnaryDecl {right,op:$o}));
    }}}

    match first {
      // 一元运算符
      b'-' => match_unary!(b'-'),
      b'!' => match_unary!(b'!'),

      // 解析字符字面量
      b'"' => {
        i += 1;
        while self.src[i] != b'"' {
          i += 1;
          if i >= len {self.err("未闭合的\"。")}
        }
        let s = String::from_utf8_lossy(&self.src[(self.i()+1)..i]);
        self.set_i(i+1);
        return Expr::Literal(Litr::Str(Box::new(s.to_string())));
      }

      // 解析带转义的字符串
      b'`' => {
        i += 1;
        let mut start = i; // 开始结算的起点
        let mut vec = Vec::<u8>::new();

        loop {
          let c = self.src[i];
          match c {
            b'`' => break,
            b'\\'=> {
              use charts::escape;

              // 结算一次
              vec.extend_from_slice(&self.src[start..i]);

              i += 1;
              // 先测试转义换行符
              macro_rules! escape_enter {() => {{
                i += 1;
                while self.src[i] == b' ' {
                  i += 1;
                }
              }}}
              let escaper = self.src[i];
              match escaper {
                b'\r'=> {
                  i += 1;
                  escape_enter!();
                }
                b'\n'=> escape_enter!(),
                // 非换行符就按转义表转义
                _=> {
                  let escaped = escape(escaper);
                  if escaped == 255 {
                    self.err(&format!("错误的转义符:{}", String::from_utf8_lossy(&[escaper])));
                  }
                  vec.push(escaped);
                  i += 1;
                }
              }

              // 更新结算起点
              start = i;
            }
            _=> i += 1
          }
          if i >= len {self.err("未闭合的`。")}
        }

        // 结算 结算起点到末尾
        vec.extend_from_slice(&self.src[start..i]);
        let str = String::from_utf8(vec)
          .expect(&format!("字符串含非法字符 解析错误({})",self.line()));

        self.set_i(i + 1);
        return Expr::Literal(Litr::Str(Box::new(str)));
      }

      // 解析'buffer'
      b'\'' => {
        i += 1;
        let mut start = i; // 开始结算的起点
        let mut vec = Vec::<u8>::new();

        /// 解析{hex}
        /// 
        /// 用宏是因为嵌套太深了看着很难受
        macro_rules! parse_hex {() => {{
          // 结算左大括号之前的内容
          vec.extend_from_slice(&self.src[start..i]);
          i += 1;
          let mut braced = i; // 大括号的起点
          // 以i为界限，把hex部分切出来
          loop {
            let char = self.src[i];
            match char {
              b'0'..=b'9'|b'a'..=b'f'|b'A'..=b'F'|b'\n'|b'\r'|b' ' => i += 1,
              b'}' => break,
              _=> self.err(&format!("十六进制非法字符:{}",String::from_utf8_lossy(&[char])))
            };
            if i >= len {self.err("未闭合的}")}
          };

          // 结算起点延后到大括号后面
          start = i + 1;

          // 处理hex
          let mut hex = Vec::with_capacity(i-braced);
          while braced < i {
            // 清除空格
            while matches!(self.src[braced],b'\n'|b'\r'|b' ') {
              braced += 1;
              if braced >= i {break}
            };
            if braced >= i {
              self.err("未闭合的}")
            }

            let res:Result<u8,_>;
            let a = self.src[braced];
            if braced >= i {break;}

            braced += 1;
            if braced < i {
              let b = self.src[braced];
              braced += 1;
              res = u8::from_str_radix(&String::from_utf8_lossy(&[a,b]), 16);
            }else {
              res = u8::from_str_radix(&String::from_utf8_lossy(&[a]), 16)
            }

            match res {
              Ok(n)=> hex.push(n),
              Err(_)=> self.err("十六进制解析:不要把一个Byte的两个字符拆开")
            }
          }
          vec.append(&mut hex);
        }}}

        loop {
          let char = self.src[i];
          match char {
            b'\'' => break,
            // 十六进制解析
            b'{' => parse_hex!(),
            _=> i += 1
          }
          if i >= len {self.err("未闭合的'。")}
        }
        // 结算 结算起点到末尾
        vec.extend_from_slice(&self.src[start..i]);

        self.set_i(i+1);
        return Expr::Literal(Litr::Buffer(Box::new(Buf::U8(vec))));
      }

      // 解析数字字面量
      b'0'..=b'9' => {
        let mut is_float = false;
        while i < len {
          match self.src[i] {
            b'.'=> is_float = true,
            0x30..=0x39 | b'e' | b'E' => {}
            _=> break
          }
          i += 1;
        }

        let str = String::from_utf8(self.src[self.i()..i].to_vec()).unwrap();
        use Litr::*;
        macro_rules! parsed {
          ($t:ty, $i:ident) => {{
            let n: Result<$t,_> = str.parse();
            match n {
              Err(e)=> {
                panic!("无法解析数字:{} 解析错误({})\n  {}",str,self.line(),e)
              }
              Ok(n)=> {
                self.next();
                return Expr::Literal($i(n));
              }
            }
          }};
        }

        self.set_i(i);
        if i < len {
          let cur = self.src[i];
          match cur {
            b'l' => parsed!(f64, Float),
            b'u' => parsed!(usize, Uint),
            b'i'=> parsed!(isize, Int),
            _=> {}
          }
        }
        self.set_i(i-1);

        if is_float {
          parsed!(f64, Float);
        }
        parsed!(isize, Int);
      },

      // 解析Buffer
      b'['=> {
        self.next();
        self.spaces();

        let expr = self.expr();

        self.spaces();
        if self.i() >= self.src.len() || self.cur() != b']' {
          self.err("未闭合的右括号']'。");
        }
        self.next();

        // 判断类型
        let ty = if self.cur() == b'(' {
          self.next();
          let id = self.ident();
          if let Some(t) = id {
            if self.cur() != b')' {
              self.err("Buffer类型声明右括号缺失")
            }
            self.next();
            if t == b"any" {
              if let Expr::Empty = expr {
                return Expr::Literal(Litr::Array(Box::new(Vec::new())));
              }
              return expr;
            }
            t
          }else {
            self.err("Buffer的类型声明为空")
          }
        }else {
          b"u8"
        };
        // Empty有机会被传进运行时，将被解析为空数组
        // 不在这里返回空数组是因为类型需要在运行时解析
        Expr::Buffer(Box::new(BufDecl{expr,ty:ty.to_vec()}))
      }

      // 解析字面量或变量
      _=> {
        let id_res = self.ident();
        if let Some(id) = id_res {
          match &*id {
            b"true"=> Expr::Literal(Litr::Bool(true)),
            b"false"=> Expr::Literal(Litr::Bool(false)),
            b"uninit"=> Expr::Literal(Litr::Uninit),
            _=> Expr::Literal(Litr::Variant(intern(id)))
          }
        }else {
          Expr::Empty
        }
      }
    }
  }

  /// 解析函数声明的参数
  fn arguments(&self)-> Vec::<(Interned,KsType)> {
    self.spaces();
    let mut args = Vec::<(Interned,KsType)>::new();
    while let Some(n) = self.ident() {
      let arg = intern(n);
      let typ:KsType = if self.cur() == b':' {
        self.next();
        let t = self.ident().unwrap_or_else(||self.err("类型声明不能为空"));
        charts::kstype(t)
      }else {
        KsType::Any
      };
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
