use super::*;
use crate::{
  native::NativeInstance, 
  runtime::{calc::CalcRef, Module, Scope},
  intern::Interned,
  scan::stmt::ClassDef,
  primitive::litr::{Litr, LocalFuncRaw}
};


impl Scanner<'_> {
  /// 解析一段字面量
  /// 
  /// 同时解析一元运算符
  pub fn literal(&self)-> Expr {
    let first = self.cur();
    let len = self.src.len();
    let mut i = self.i();
  
    match first {
      // 解析字符字面量
      b'"' => {
        i += 1;
        while self.src[i] != b'"' {
          i += 1;
          assert!(i < len, "未闭合的\"。");
        }
        let s = String::from_utf8_lossy(&self.src[(self.i()+1)..i]);
        self.set_i(i+1);
        Expr::Literal(Litr::Str(s.to_string()))
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
                  let escaped = charts::escape(escaper);
                  if escaped == 255 {
                    panic!("错误的转义符:{}", String::from_utf8_lossy(&[escaper]));
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
          if i >= len {panic!("未闭合的'`'。")}
        }
  
        // 结算 结算起点到末尾
        vec.extend_from_slice(&self.src[start..i]);
        let str = match String::from_utf8(vec) {
          Ok(s)=> s,
          Err(_)=> panic!("字符串含非法字符")
        };

        self.set_i(i + 1);
        Expr::Literal(Litr::Str(str))
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
              _=> panic!("十六进制非法字符:{}",String::from_utf8_lossy(&[char]))
            };
            if i >= len {panic!("未闭合的}}")}
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
              panic!("未闭合的}}")
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
              Err(_)=> panic!("十六进制解析:不要把一个Byte的两个字符拆开")
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
          if i >= len {panic!("未闭合的'。")}
        }
        // 结算 结算起点到末尾
        vec.extend_from_slice(&self.src[start..i]);
  
        self.set_i(i+1);
        Expr::Literal(Litr::Buf(vec))
      }
  
      // 解析数字字面量
      b'0'..=b'9' => {
        let mut is_float = false;
        while i < len {
          match self.src[i] {
            b'.'=> {
              if is_float ||
              // 判断下一个字符是否数字
                (i+1 < len && !(0x30..=0x39).contains(&self.src[i+1]))
              {
                break;
              }
              is_float = true
            },
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
                panic!("无法解析数字:{}\n  {}",str,e)
              }
              Ok(n)=> {
                self.next();
                return Expr::Literal($i(n));
              }
            }
          }}
        }
  
        self.set_i(i);
        if i < len {
          let cur = self.src[i];
          match cur {
            b'f' => parsed!(f64, Float),
            b'u' => parsed!(usize, Uint),
            b'i'=> parsed!(isize, Int),
            _=> {}
          }
        }
        self.set_i(i-1);
  
        if is_float {
          parsed!(f64, Float)
        }else {
          parsed!(isize, Int)
        }
      },
  
      // 解析List
      b'['=> {
        self.next();
        self.spaces();
  
        let mut ls = Vec::new();
        loop {
          let e = self.expr();
          if let Expr::Empty = e {
            break;
          }
          ls.push(e);
          self.spaces();
          if self.cur() != b',' {
            break;
          }
          self.next();
        }
        if self.i() >= self.src.len() || self.cur() != b']' {
          if self.cur() == b',' {
            panic!("列表不允许空元素");
          }
          panic!("未闭合的右括号']'。");
        }
        self.next();
        Expr::List(ls)
      }

      // 解析对象
      b'{'=> Expr::Obj(self.obj()),

      // 解析闭包或管道占位符
      b'|'=> {
        self.next();

        // 遇到管道占位符时 直接将管道暂存的表达式返回
        if self.cur()==b'%' {
          self.next();
          self.next();
          return unsafe {
            super::expr::ON_PIPE.take().expect("管道占位符只能在管道操作符'|>'后使用")
          };
        }
        
        // 解析闭包参数
        let args = self.arguments();
        assert!(self.cur()==b'|', "闭包声明右括号缺失");
        self.next();

        // 解析闭包内容
        let stmt = self.stmt();
        let mut stmts = if let super::Stmt::Block(b) = stmt {
          b
        }else {
          Statements(vec![(unsafe{crate::LINE}, stmt)])
        };

        Expr::LocalDecl(LocalFuncRaw { argdecl: args, stmts })
      }
  
      // 解析字面量或变量
      _=> {
        let id_res = self.ident();
        if let Some(id) = id_res {
          match &*id {
            b"true"=> Expr::Literal(Litr::Bool(true)),
            b"false"=> Expr::Literal(Litr::Bool(false)),
            b"self"=> Expr::Kself,
            b"uninit"=> Expr::Literal(Litr::Uninit),
            _=> Expr::Variant(intern(id))
          }
        }else {
          Expr::Empty
        }
      }
    }
  }

  /// 解析对象表达式
  fn obj(&self)-> Vec<(Interned,Expr)> {
    self.next();
    self.spaces();
    let mut decl = Vec::new();
    while let Some(id) = self.ident() {
      let v = if self.cur() == b':' {
        self.next();
        self.expr()
      }else {Expr::Literal(Litr::Uninit)};
      decl.push((intern(id),v));

      if self.cur() == b',' {
        self.next()
      }
      self.spaces();
    }

    if self.cur() != b'}' {
      panic!("未闭合的大括号")
    };
    self.next();
    decl
  }
}

