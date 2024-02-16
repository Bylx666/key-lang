use super::{Scanner, scan};
use crate::intern::{Interned,intern};
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
  Let       (Box<AssignDef>),

  // 定义类
  Class     (Box<ClassDefRaw>),
  // 类别名
  Using     (Box<(Interned, Expr)>),

  Mod       (Box<LocalModDef>),
  ExportFn  (Box<(Interned, LocalFuncRaw)>),
  ExportCls (Box<ClassDefRaw>),

  // Key
  // Key       (HashMap<Ident, KsType>),                // 类型声明语句
  // Impl      (HashMap<Ident, KsLocalFunc>), // 方法定义语句
  Match,     // 模式匹配

  // 块系列
  Block    (Box<Statements>),   // 一个普通块
  If       (Box<Statements>),   // 条件语句
  Loop     (Box<Statements>),   // 循环

  // 流程控制
  Break     (Box<Expr>),                  // 中断循环并提供返回值
  Continue,                           // 立刻进入下一次循环
  Return    (Box<Expr>),                  // 函数返回

  // 表达式作为语句
  Expression(Box<Expr>),
}

/// 赋值语句
#[derive(Debug, Clone)]
pub struct AssignDef {
  pub id: Interned,
  pub val: Expr
}


#[derive(Debug, Clone)]
pub struct LocalModDef {
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
  pub module: *mut Module
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

#[derive(Debug, Clone)]
pub struct NativeMod {
  pub name: Interned,
  pub funcs: Vec<crate::native::NativeFn>
}


pub(super) fn stmt(this:&Scanner)-> Stmt {
  this.spaces();
  if this.i() >= this.src.len() {
    this.next(); // 打破scan函数的while
    return Stmt::Empty;
  }

  let first = this.cur();
  match first {
    // 分号开头即为空语句
    b';' => {
      this.next();
      return Stmt::Empty;
    }
    // 块语句
    b'{' => {
      let mut stmts = Statements::default();
      let len = this.src.len();
      this.next();
      loop {
        if this.i() >= len {
          this.err("未闭合的块大括号");
        }
        this.spaces();
        if this.cur() == b'}' {
          this.next();
          return Stmt::Block(Box::new(stmts));
        }
        
        let s = this.stmt();
        if let Stmt::Empty = s {
          continue;
        }
        stmts.0.push((this.line(), s));
      }
    }
    // 返回语句语法糖
    b':' => {
      this.next();
      return returning(this);
    }
    _=>{}
  }

  let ident = this.ident();
  if let Some(id) = ident {
    match id {
      // 如果是关键词，就会让对应函数处理关键词之后的信息
      b"let"=> Stmt::Let(letting(this)),
      b"extern"=> {externing(this);Stmt::Empty},
      b"return"=> returning(this),
      b"class"=> classing(this),
      b"mod"=> moding(this),
      b"async"|b"await"=> this.err("异步关键词暂未实现"),
      _=> {
        let left = match &*id {
          b"true"=> Expr::Literal(Litr::Bool(true)),
          b"false"=> Expr::Literal(Litr::Bool(false)),
          b"self"=> Expr::Kself,
          b"uninit"=> Expr::Literal(Litr::Uninit),
          _=> Expr::Variant(intern(id))
        };
        let expr = Box::new(this.expr_with_left(left));
        Stmt::Expression(expr)
      }
    }
  }else {
    let expr = this.expr();
    if let Expr::Empty = expr {
      this.err(&format!("请输入一行正确的语句，'{}'并不合法", String::from_utf8_lossy(&[this.cur()])))
    }
    Stmt::Expression(Box::new(expr))
  }
}


/// 解析let关键词
fn letting(this:&Scanner)-> Box<AssignDef> {
  this.spaces();
  let id = this.ident().unwrap_or_else(||this.err("let后需要标识符"));
  let id = intern(id);

  // 检查标识符后的符号
  this.spaces();
  let sym = this.cur();
  match sym {
    b'=' => {
      this.next();
      let val = this.expr();
      if let Expr::Empty = val {
        this.err("无法为空气赋值")
      }
      Box::new(AssignDef {
        id, val
      })
    }
    b'(' => {
      this.next();
      let args = this.arguments();
      if this.cur() != b')' {
        this.err("函数声明右括号缺失");
      }
      this.next();

      let stmt = this.stmt();
      let mut stmts = if let Stmt::Block(b) = stmt {
        *b
      }else {
        Statements(vec![(this.line(), stmt)])
      };

      // scan过程产生的LocalFunc是没绑定作用域的，因此不能由运行时来控制其内存释放
      // 其生命周期应当和Statements相同，绑定作用域时将被复制
      // 绑定作用域行为发生在runtime::Scope::calc
      let func = Box::new(LocalFuncRaw { argdecl: args, stmts });
      Box::new(AssignDef {
        id, 
        val: Expr::LocalDecl(func)
      })
    }
    _ => Box::new(AssignDef {
      id, val:Expr::Literal(Litr::Uninit)
    })
  }
}

/// extern关键词
/// 
/// 会固定返回空语句
fn externing(this:&Scanner) {
  use crate::c::Clib;

  // 截取路径
  this.spaces();
  let mut i = this.i();
  let len = this.src.len();
  while this.src[i] != b'>' {
    if i >= len {
      this.err("extern后需要 > 符号");
    }
    i += 1;
  }

  let path = &this.src[this.i()..i];
  let lib = Clib::load(path).unwrap_or_else(|e|this.err(&e));
  this.set_i(i + 1);
  this.spaces();

  /// 解析并推走一个函数声明
  macro_rules! parse_decl {($id:ident) => {{
    let sym:&[u8];
    // 别名解析
    if this.cur() == b':' {
      this.next();
      this.spaces();
      if let Some(i) = this.ident() {
        sym = i;
      }else {
        this.err(":后需要别名")
      };
    }else {
      sym = $id;
    }

    // 解析小括号包裹的参数声明
    if this.cur() != b'(' {
      this.err("extern函数后应有括号");
    }
    this.next();
    let argdecl = this.arguments();
    this.spaces();
    if this.cur() != b')' {
      this.err("extern函数声明右括号缺失");
    }
    this.next();
    
    if this.cur() == b';' {
      this.next();
    }

    // 将函数名(id)和指针(ptr)作为赋值语句推到语句列表里
    let ptr = lib.get(sym).unwrap_or_else(||this.err(
      &format!("动态库'{}'中不存在'{}'函数", 
      String::from_utf8_lossy(path), 
      String::from_utf8_lossy(sym))));
    this.push(Stmt::Let(Box::new(AssignDef { 
      id:intern($id), 
      val: Expr::Literal(Litr::Func(Box::new(Function::Extern(Box::new(ExternFunc { 
        argdecl, 
        ptr
      })))))
    })));
  }}}


  // 大括号语法
  if this.cur() == b'{' {
    this.next();
    this.spaces();
    while let Some(id) = this.ident() {
      parse_decl!(id);
      this.spaces();
    }
    this.spaces();
    if this.cur() != b'}' {
      this.err("extern大括号未闭合")
    }
    this.next();
  }else {
    // 省略大括号语法
    let id = this.ident().unwrap_or_else(||this.err("extern后应有函数名"));
    parse_decl!(id);
  }
}

/// 解析返回语句
fn returning(this:&Scanner)-> Stmt {
  this.spaces();
  let expr = this.expr();
  if let Expr::Empty = expr {
    Stmt::Return(Box::new(Expr::Literal(Litr::Uninit)))
  }else {
    Stmt::Return(Box::new(expr))
  }
}


/// 解析类声明
fn classing(this:&Scanner)-> Stmt {
  this.spaces();
  let id = this.ident().unwrap_or_else(||this.err("class后需要标识符"));
  this.spaces();
  if this.cur() == b'=' {
    this.next();
    let right = this.expr();
    return Stmt::Using(Box::new((intern(id),right)));
  }
  if this.cur() != b'{' {
    this.err("class需要大括号");
  }
  this.next();

  let mut props = Vec::new();
  let mut methods = Vec::new();
  let mut statics = Vec::new();
  loop {
    this.spaces();
    let public = if this.cur() == b'>' {
      this.next();this.spaces();true
    }else {false};
    
    let is_method = if this.cur() == b'.' {
      this.next();true
    }else {false};

    let id = match this.ident() {
      Some(id)=> id,
      None=> break
    };

    // 方法或者函数
    if this.cur() == b'(' {
      this.next();
      // 参数
      let args = this.arguments();
      if this.cur() != b')' {
        this.err("函数声明右括号缺失??");
      }
      this.next();

      // 函数体
      let stmt = this.stmt();
      let mut stmts = if let Stmt::Block(b) = stmt {
        *b
      }else {
        Statements(vec![(this.line(), stmt)])
      };

      let v = ClassFuncRaw {name: intern(id), f:LocalFuncRaw{argdecl:args,stmts}, public};
      if is_method {
        methods.push(v);
      }else {
        statics.push(v);
      }
    // 属性
    }else {
      let typ = this.typ();
      let v = ClassProp {
        name: intern(id), typ, public
      };
      props.push(v);
    }

    this.spaces();
    if this.cur() == b',' {
      this.next();
    }
    this.spaces();
  }

  this.spaces();
  if this.cur() != b'}' {
    this.err("class大括号未闭合");
  }
  this.next();
  Stmt::Class(Box::new(ClassDefRaw {
    name:intern(id), props, methods, statics
  }))
}


/// 解析模块声明
fn moding(this:&Scanner)-> Stmt {
  // 先判断是否是导出语句
  match this.cur() {
    b'.' => {
      this.next();
      // 套用let声明模板
      let asn = letting(this);
      if let Expr::LocalDecl(f) = asn.val {
        return Stmt::ExportFn(Box::new((asn.id, (*f).clone())));
      }
      this.err("模块只能导出本地函数。\n  若导出外界函数请用本地函数包裹。")
    },
    b':' => {
      this.next();
      let cls = classing(this);
      match cls {
        Stmt::Class(cls)=> return Stmt::ExportCls(cls),
        Stmt::Using(acc)=> this.err("无法导出using"),
        _=> unreachable!()
      }
    }
    _=>{}
  };
  // 截取路径
  this.spaces();
  let mut i = this.i();
  let len = this.src.len();
  let mut dot = 0;
  loop {
    if i >= len {
      this.err("mod后需要 > 符号");
    }
    let cur = this.src[i];
    if cur == b'>' {
      break;
    }
    if cur == b'.' {
      dot = i;
    }
    i += 1;
  }

  let path = &this.src[this.i()..i];
  this.set_i(i + 1);

  this.spaces();
  let name = intern(&this.ident().unwrap_or_else(||this.err("需要为模块命名")));
  this.spaces();

  if dot == 0 {
    this.err("未知模块类型")
  }
  let suffix = &this.src[dot..i];
  match suffix {
    b".ksm"|b".dll"=> {
      let module = crate::native::parse(name, path).unwrap_or_else(|e|
        this.err(&format!("模块解析失败:{}\n  {}",e,String::from_utf8_lossy(path))));
      Stmt::Mod(Box::new(module))
    }
    b".ks"=> {
      let path = &*String::from_utf8_lossy(path);
      let file = std::fs::read(path).unwrap_or_else(|e|this.err(&format!(
        "无法找到模块'{}'", path
      )));
      let mut module = crate::runtime::run(&scan(file)).exported;
      module.name = name;
      Stmt::Mod(Box::new(module))
    }
    _ => this.err("未知模块类型")
  }
}