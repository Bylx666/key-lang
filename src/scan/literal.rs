use super::{
  charts, intern, ObjDecl, Scanner
};
use crate::{ast::{
  Expr, Litr, UnaryDecl
}, scan::NewDecl};
pub fn literal(this:&Scanner)-> Expr {
  let first = this.cur();
  let len = this.src.len();
  let mut i = this.i();

  macro_rules! match_unary {($o:expr) => {{
    this.next();
    let right = this.literal();
    Expr::Unary(Box::new(UnaryDecl {right,op:$o}))
  }}}

  match first {
    // 一元运算符
    b'-' => match_unary!(b'-'),
    b'!' => match_unary!(b'!'),

    // 解析字符字面量
    b'"' => {
      i += 1;
      while this.src[i] != b'"' {
        i += 1;
        if i >= len {this.err("未闭合的\"。")}
      }
      let s = String::from_utf8_lossy(&this.src[(this.i()+1)..i]);
      this.set_i(i+1);
      Expr::Literal(Litr::Str(Box::new(s.to_string())))
    }

    // 解析带转义的字符串
    b'`' => {
      i += 1;
      let mut start = i; // 开始结算的起点
      let mut vec = Vec::<u8>::new();

      loop {
        let c = this.src[i];
        match c {
          b'`' => break,
          b'\\'=> {
            use charts::escape;

            // 结算一次
            vec.extend_from_slice(&this.src[start..i]);

            i += 1;
            // 先测试转义换行符
            macro_rules! escape_enter {() => {{
              i += 1;
              while this.src[i] == b' ' {
                i += 1;
              }
            }}}
            let escaper = this.src[i];
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
                  this.err(&format!("错误的转义符:{}", String::from_utf8_lossy(&[escaper])));
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
        if i >= len {this.err("未闭合的`。")}
      }

      // 结算 结算起点到末尾
      vec.extend_from_slice(&this.src[start..i]);
      let str = String::from_utf8(vec)
        .expect(&format!("字符串含非法字符 解析错误({})",this.line()));

      this.set_i(i + 1);
      Expr::Literal(Litr::Str(Box::new(str)))
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
        vec.extend_from_slice(&this.src[start..i]);
        i += 1;
        let mut braced = i; // 大括号的起点
        // 以i为界限，把hex部分切出来
        loop {
          let char = this.src[i];
          match char {
            b'0'..=b'9'|b'a'..=b'f'|b'A'..=b'F'|b'\n'|b'\r'|b' ' => i += 1,
            b'}' => break,
            _=> this.err(&format!("十六进制非法字符:{}",String::from_utf8_lossy(&[char])))
          };
          if i >= len {this.err("未闭合的}")}
        };

        // 结算起点延后到大括号后面
        start = i + 1;

        // 处理hex
        let mut hex = Vec::with_capacity(i-braced);
        while braced < i {
          // 清除空格
          while matches!(this.src[braced],b'\n'|b'\r'|b' ') {
            braced += 1;
            if braced >= i {break}
          };
          if braced >= i {
            this.err("未闭合的}")
          }

          let res:Result<u8,_>;
          let a = this.src[braced];
          if braced >= i {break;}

          braced += 1;
          if braced < i {
            let b = this.src[braced];
            braced += 1;
            res = u8::from_str_radix(&String::from_utf8_lossy(&[a,b]), 16);
          }else {
            res = u8::from_str_radix(&String::from_utf8_lossy(&[a]), 16)
          }

          match res {
            Ok(n)=> hex.push(n),
            Err(_)=> this.err("十六进制解析:不要把一个Byte的两个字符拆开")
          }
        }
        vec.append(&mut hex);
      }}}

      loop {
        let char = this.src[i];
        match char {
          b'\'' => break,
          // 十六进制解析
          b'{' => parse_hex!(),
          _=> i += 1
        }
        if i >= len {this.err("未闭合的'。")}
      }
      // 结算 结算起点到末尾
      vec.extend_from_slice(&this.src[start..i]);

      this.set_i(i+1);
      Expr::Literal(Litr::Buffer(Box::new(vec)))
    }

    // 解析数字字面量
    b'0'..=b'9' => {
      let mut is_float = false;
      while i < len {
        match this.src[i] {
          b'.'=> is_float = true,
          0x30..=0x39 | b'e' | b'E' => {}
          _=> break
        }
        i += 1;
      }

      let str = String::from_utf8(this.src[this.i()..i].to_vec()).unwrap();
      use Litr::*;
      macro_rules! parsed {
        ($t:ty, $i:ident) => {{
          let n: Result<$t,_> = str.parse();
          match n {
            Err(e)=> {
              panic!("无法解析数字:{} 解析错误({})\n  {}",str,this.line(),e)
            }
            Ok(n)=> {
              this.next();
              return Expr::Literal($i(n));
            }
          }
        }}
      }

      this.set_i(i);
      if i < len {
        let cur = this.src[i];
        match cur {
          b'l' => parsed!(f64, Float),
          b'u' => parsed!(usize, Uint),
          b'i'=> parsed!(isize, Int),
          _=> {}
        }
      }
      this.set_i(i-1);

      if is_float {
        parsed!(f64, Float)
      }else {
        parsed!(isize, Int)
      }
    },

    // 解析List
    b'['=> {
      this.next();
      this.spaces();

      let mut ls = Vec::new();
      loop {
        let e = this.expr();
        if let Expr::Empty = e {
          break;
        }
        ls.push(e);
        this.spaces();
        if this.cur() != b',' {
          break;
        }
        this.next();
      }
      if this.i() >= this.src.len() || this.cur() != b']' {
        if this.cur() == b',' {
          this.err("列表不允许空元素");
        }
        this.err("未闭合的右括号']'。");
      }
      this.next();
      Expr::List(Box::new(ls))
    }

    // 解析对象
    b'{'=> Expr::Obj(Box::new(obj(this))),

    // 解析字面量或变量
    _=> {
      let id_res = this.ident();
      if let Some(id) = id_res {
        match &*id {
          b"true"=> Expr::Literal(Litr::Bool(true)),
          b"false"=> Expr::Literal(Litr::Bool(false)),
          b"self"=> Expr::Kself,
          b"uninit"=> Expr::Literal(Litr::Uninit),
          _=> {
            this.spaces();
            if this.cur() == b'{' {
              if id[0].is_ascii_uppercase() {
                let decl = obj(this);
                return Expr::NewInst(Box::new(NewDecl {
                  cls: intern(id),
                  val: decl
                }));
              }
            }
            Expr::Variant(intern(id))
          }
        }
      }else {
        Expr::Empty
      }
    }
  }
}


/// 解析对象表达式
fn obj(this:&Scanner)-> ObjDecl {
  this.next();
  this.spaces();
  let mut decl = Vec::new();
  while let Some(id) = this.ident() {
    let v = if this.cur() == b':' {
      this.next();
      this.expr()
    }else {Expr::Literal(Litr::Uninit)};
    decl.push((intern(id),v));

    if this.cur() == b',' {
      this.next()
    }
    this.spaces();
  }

  if this.cur() != b'}' {
    this.err("未闭合的大括号")
  };
  this.next();
  ObjDecl (decl)
}