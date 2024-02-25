use super::{Scanner, scan};
use crate::intern::{Interned,intern};
use crate::native::NativeMod;
use crate::runtime::{Scope, ScopeInner, Module};
use super::{
  literal::{Litr, Function, LocalFuncRaw, LocalFunc, ExternFunc, KsType},
  expr::Expr
};

/// 语句列表
#[derive(Debug, Clone, Default)]
pub struct Statements (
  pub Vec<(usize, Stmt)>
);

/// 分号分隔的，statement语句
#[derive(Debug, Clone)]
pub enum Stmt {
  Empty,

  // 赋值
  Let       (AssignDef),

  // 定义类
  Class     (ClassDefRaw),
  // 类别名
  Using     (Interned, Expr),

  Mod       (LocalMod),
  NativeMod (*const NativeMod),
  ExportFn  (Interned, LocalFuncRaw),
  ExportCls (ClassDefRaw),

  Match,     // 模式匹配

  // 块系列
  Block    (Statements),   // 一个普通块
  If {
    condition: Expr,
    exec: Box<Stmt>,
    els: Option<Box<Stmt>>
  },
  ForLoop,
  ForWhile {
    condition: Expr,
    exec: Box<Stmt>
  },
  ForIter,
  // If       (Statements),   // 条件语句
  // Loop     (Statements),   // 循环

  // 流程控制
  Break,
  Continue,                           // 立刻进入下一次循环
  Return    (Expr),                  // 函数返回

  // 表达式作为语句
  Expression(Expr),
}

/// 赋值语句
#[derive(Debug, Clone)]
pub struct AssignDef {
  pub id: Interned,
  pub val: Expr
}


#[derive(Debug, Clone)]
pub struct LocalMod {
  pub name: Interned,
  pub funcs: Vec<(Interned, LocalFunc)>,
  pub classes: Vec<(Interned, *const ClassDef)>
}

/// 未绑定作用域的类声明
#[derive(Debug, Clone)]
pub struct ClassDefRaw {
  pub name: Interned,
  pub props: Vec<ClassProp>,
  pub methods: Vec<ClassFuncRaw>,
  pub statics: Vec<ClassFuncRaw>
}

/// 绑定作用域的类声明
#[derive(Debug, Clone)]
pub struct ClassDef {
  pub name: Interned,
  pub props: Vec<ClassProp>,
  pub statics: Vec<ClassFunc>,
  pub methods: Vec<ClassFunc>,
  /// 用来判断是否在模块外
  pub module: *mut LocalMod
}

/// 类中的属性声明
#[derive(Debug, Clone)]
pub struct ClassProp {
  pub name: Interned,
  pub typ: KsType,
  pub public: bool
}

/// 类中的未绑定作用域的函数声明
#[derive(Debug,Clone)]
pub struct ClassFuncRaw {
  pub name: Interned,
  pub f: LocalFuncRaw,
  pub public: bool
}

/// 类中的函数声明
#[derive(Debug,Clone)]
pub struct ClassFunc {
  pub name: Interned,
  pub f: LocalFunc,
  pub public: bool
}


impl Scanner<'_> {
  /// 匹配一个语句
  pub fn stmt(&self)-> Stmt {
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
            return Stmt::Block(stmts);
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
  
    let ident = self.literal();
    if let Expr::Variant(id) = ident {
      match &*id.vec() {
        // 如果是关键词，就会让对应函数处理关键词之后的信息
        b"let"=> Stmt::Let(self.letting()),
        b"extern"=> {self.externing();Stmt::Empty},
        b"return"=> self.returning(),
        b"class"=> self.classing(),
        b"mod"=> self.moding(),
        b"for"=> self.foring(),
        b"if"=> self.ifing(),
        b"break"=> Stmt::Break,
        b"continue"=> Stmt::Continue,
        b"async"|b"await"=> self.err("异步关键词暂未实现"),
        _=> {
          let expr = self.expr_with_left(ident);
          Stmt::Expression(expr)
        }
      }
    }else {
      let expr = self.expr_with_left(ident);
      Stmt::Expression(expr)
    }
  }

  /// 解析let关键词
  fn letting(&self)-> AssignDef {
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
        AssignDef {
          id, val
        }
      }
      b'(' => {
        self.next();
        let args = self.arguments();
        if self.cur() != b')' {
          self.err("函数声明右括号缺失");
        }
        self.next();
  
        let stmt = self.stmt();
        let mut stmts = if let Stmt::Block(b) = stmt {
          b
        }else {
          Statements(vec![(self.line(), stmt)])
        };
  
        // scan过程产生的LocalFunc是没绑定作用域的，因此不能由运行时来控制其内存释放
        // 其生命周期应当和Statements相同，绑定作用域时将被复制
        // 绑定作用域行为发生在runtime::Scope::calc
        AssignDef {
          id, 
          val: Expr::LocalDecl(LocalFuncRaw { argdecl: args, stmts })
        }
      }
      _ => AssignDef {
        id, val:Expr::Literal(Litr::Uninit)
      }
    }
  }
  
  /// extern关键词
  /// 
  /// 会固定返回空语句
  fn externing(&self) {
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
      self.push(Stmt::Let(AssignDef { 
        id:intern($id), 
        val: Expr::Literal(Litr::Func(Function::Extern(ExternFunc { 
          argdecl, 
          ptr
        })))
      }));
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
    }else {
      // 省略大括号语法
      let id = self.ident().unwrap_or_else(||self.err("extern后应有函数名"));
      parse_decl!(id);
    }
  }

  /// 解析返回语句
  fn returning(&self)-> Stmt {
    self.spaces();
    let expr = self.expr();
    if let Expr::Empty = expr {
      Stmt::Return(Expr::Literal(Litr::Uninit))
    }else {
      Stmt::Return(expr)
    }
  }

  /// 解析类声明
  fn classing(&self)-> Stmt {
    self.spaces();
    let id = self.ident().unwrap_or_else(||self.err("class后需要标识符"));
    self.spaces();
    if self.cur() == b'=' {
      self.next();
      let right = self.expr();
      return Stmt::Using(intern(id),right);
    }
    if self.cur() != b'{' {
      self.err("class需要大括号");
    }
    self.next();
  
    let mut props = Vec::new();
    let mut methods = Vec::new();
    let mut statics = Vec::new();
    loop {
      self.spaces();
      let public = if self.cur() == b'>' {
        self.next();self.spaces();true
      }else {false};
      
      let is_method = if self.cur() == b'.' {
        self.next();true
      }else {false};
  
      let id = match self.ident() {
        Some(id)=> id,
        None=> break
      };
  
      // 方法或者函数
      if self.cur() == b'(' {
        self.next();
        // 参数
        let args = self.arguments();
        if self.cur() != b')' {
          self.err("函数声明右括号缺失??");
        }
        self.next();
  
        // 函数体
        let stmt = self.stmt();
        let mut stmts = if let Stmt::Block(b) = stmt {
          b
        }else {
          Statements(vec![(self.line(), stmt)])
        };
  
        let v = ClassFuncRaw {name: intern(id), f:LocalFuncRaw{argdecl:args,stmts}, public};
        if is_method {
          methods.push(v);
        }else {
          statics.push(v);
        }
      // 属性
      }else {
        let typ = self.typ();
        let v = ClassProp {
          name: intern(id), typ, public
        };
        props.push(v);
      }
  
      self.spaces();
      if self.cur() == b',' {
        self.next();
      }
      self.spaces();
    }
  
    self.spaces();
    if self.cur() != b'}' {
      self.err("class大括号未闭合");
    }
    self.next();
    Stmt::Class(ClassDefRaw {
      name:intern(id), props, methods, statics
    })
  }
  
  
  /// 解析模块声明
  fn moding(&self)-> Stmt {
    // 先判断是否是导出语句
    match self.cur() {
      b'.' => {
        self.next();
        // 套用let声明模板
        let asn = self.letting();
        if let Expr::LocalDecl(f) = asn.val {
          return Stmt::ExportFn(asn.id, f.clone());
        }
        self.err("模块只能导出本地函数。\n  若导出外界函数请用本地函数包裹。")
      },
      b':' => {
        self.next();
        let cls = self.classing();
        match cls {
          Stmt::Class(cls)=> return Stmt::ExportCls(cls),
          Stmt::Using(_,_)=> self.err("无法导出using"),
          _=> unreachable!()
        }
      }
      _=>{}
    };
    // 截取路径
    self.spaces();
    let mut i = self.i();
    let len = self.src.len();
    let mut dot = 0;
    loop {
      if i >= len {
        self.err("mod后需要 > 符号");
      }
      let cur = self.src[i];
      if cur == b'>' {
        break;
      }
      if cur == b'.' {
        dot = i;
      }
      i += 1;
    }
  
    let path = &self.src[self.i()..i];
    self.set_i(i + 1);
  
    self.spaces();
    let name = intern(&self.ident().unwrap_or_else(||self.err("需要为模块命名")));
    self.spaces();
  
    if dot == 0 {
      self.err("未知模块类型")
    }
    let suffix = &self.src[dot..i];
    match suffix {
      b".ksm"|b".dll"=> {
        let module = crate::native::parse(name, path).unwrap_or_else(|e|
          self.err(&format!("模块解析失败:{}\n  {}",e,String::from_utf8_lossy(path))));
        Stmt::NativeMod(module)
      }
      b".ks"=> {
        let path = &*String::from_utf8_lossy(path);
        let file = std::fs::read(path).unwrap_or_else(|e|self.err(&format!(
          "无法找到模块'{}'", path
        )));
        let mut module = crate::runtime::run(&scan(file)).exports;
        module.name = name;
        Stmt::Mod(module)
      }
      _ => self.err("未知模块类型")
    }
  }

  fn ifing(&self)-> Stmt {
    let condition = self.expr();
    let exec = Box::new(self.stmt());
    self.spaces();
    if self.cur() == b'e' {

    }
    Stmt::If { condition, exec, els: None }
  }

  /// for语句
  fn foring(&self)-> Stmt {
    self.spaces();
    if self.cur() == b'(' {
      let condition = self.expr_group();
      let exec = Box::new(self.stmt());
      Stmt::ForWhile { condition, exec }
    }else {
      self.err("for语法错误")
    }
  }
}

