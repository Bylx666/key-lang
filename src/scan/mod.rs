// 将源码扫描为 AST的过程

use crate::ast::{
  keywords,
  Expr, 
  Statmnt,
  Imme, KsAssign
};
use crate::Ident;

// 将字符整理为ast
pub fn scan(src: Vec<u8>)-> Vec<Statmnt> {
  let mut scanner = Scanner {src, i:0};
  scanner.scan()
}
struct Scanner {
  src: Vec<u8>,
  i: usize
}
impl Scanner {
  fn scan(&mut self)-> Vec<Statmnt> {
    let mut stats = Vec::<Statmnt>::new();
    stats.push(self.statem());
    stats
  }

  /// 匹配一个语句
  fn statem(&mut self)-> Statmnt {
    use keywords::*;
    self.spaces();

    // 分号开头即为空语句
    if self.src[self.i] == 0x3B {return Statmnt::Empty;}

    // 判断开头是否为关键词
    let id = self.ident();
    match &*id {
      _=> {Statmnt::Expression(Box::new(self.expr(id)))}
    }

  }


  /// 匹配一段表达式，传入起始ident(如果有)
  fn expr(&mut self, ident:Vec<u8>)-> Expr {
    self.spaces();
    let first = self.src[self.i];
    let len = self.src.len();

    // 标识符非空说明index在标识符后面，就从此开始匹配
    if ident.len() != 0 {
      // 标识符后是；说明表达式结束了直接返回
      if first == 0x3B {
        self.i += 1;
        return Expr::Immediate(Imme::Variant(ident));
      }

      // 标识符后有括号就认定为call
      if first == 0x28 {
        self.i += 1;
        let mut args = Vec::new();
        loop {
          self.spaces();
          let arg = self.expr(Vec::new());
          args.push(Box::new(arg));
          self.spaces();

          // 找逗号和右括号
          let c = self.src[self.i];
          if c == 0x2C {self.i += 1;} // ,
          if c == 0x29 {break;} // )

        }
        return Expr::Call { args, targ: ident };
      }

      panic!("你需要为标识符 '{}' 后使用 ';' 结尾。{}", String::from_utf8_lossy(&ident), self.get_line_column());
    }

    // 如果ident是空的就从头匹配
    self.spaces();

    // 解析字符字面量
    if first == 0x22 {
      let mut i = self.i;
      loop {
        i += 1;
        if i >= len {panic!("未找到字符串结尾的'\"'。{}", self.get_line_column())}
        if self.src[i] == 0x22 {break;}
      }
      let s = self.src[(self.i+1)..i].to_vec();
      self.i = i + 1;
      return Expr::Immediate(Imme::Str(s));
    }
    Expr::Path


    // 数字
    // if s>=0x30 && s<=39 {
    //   loop {
    //     let c = self.src[i];
    //     if c>=0x30 && c<=0x39 {}
    //     i+=1;
    //   }
    // }

    
  }

  
  /// 匹配标识符(如果匹配不到则返回的vec.len()为0)
  fn ident(&mut self)-> Ident {
    let mut i = self.i;
    let len = self.src.len();

    // 判断首字是否为数字
    let first = self.src[i];
    if first>=0x30 && first<=0x39 {return Vec::new();}

    loop {
      if i >= len {panic!();}
      let s = self.src[i];
      if 
        (s>=0x40 && s<=0x5A) || // 大写和@
        (s>=0x61 && s<=0x7A) || // 小写
        (s>=0x30 && s<=0x39) || // 数字
        (match s {
          0x5F=> true, // _
          0x24=> true, // $
          0x7E=> true, // ~
          _=>    false
        })
      {
        i += 1;
        continue;
      }

      let ident = self.src[self.i..i].to_vec();
      self.i = i;
      return ident;
    }
  }


  /// 跳过一段空格和换行符
  fn spaces(&mut self) {
    let mut i = self.i;
    loop {
      let c = self.src[i];
      match c {
        0x20=> true,
        0x0D=> true,
        0x0A=> true,
        _=> {
          self.i = i;
          break;
        }
      };
      i += 1;
    }
  }

  /// 获取当前index的行列数
  fn get_line_column(&self)-> String {
    let mut ln = 1usize;
    let mut col = 1usize;
    let len = self.i;
    let mut i = 0;
    loop {
      if self.src[i] == 0x0A {
        ln += 1;
        col = 1;
      }
      col += 1;
      i += 1;
      if i > len {break;}
    }
    format!("(第{}行第{}列)",ln,col)
  }
}
