//! 注释都在mod.rs里，这没有注解

use super::*;

/// 解析一个表达式，对应Expr
/// 
/// 该函数必定发生复制
pub fn calc(this:&mut Scope,e:&Expr)-> Litr {
  use Litr::*;
  match e {
    Expr::Call(c)=> this.call(c),

    Expr::Literal(litr)=> litr.clone(),

    Expr::Variant(id)=> this.var(*id).clone(),

    // 函数表达式
    Expr::LocalDecl(local)=> {
      let mut f = &**local;
      let exec = Function::Local(Box::new(LocalFunc::new(f, *this)));
      Litr::Func(Box::new(exec))
    }

    // 二元运算符
    Expr::Binary(bin)=> binary(this, bin),

    // 一元运算符
    Expr::Unary(una)=> {
      let right = this.calc_ref(&una.right);
      match una.op {
        b'-'=> {
          match &*right {
            Int(n)=> Int(-n),
            Float(n)=> Float(-n),
            _=> err("负号只能用在有符号数")
          }
        }
        b'!'=> {
          match &*right {
            Bool(b)=> Bool(!b),
            Int(n)=> Int(!n),
            Uint(n)=> Uint(!n),
            Uninit => Bool(true),
            _=> err("!运算符只能用于整数和Bool")
          }
        }_=>Uninit
      }
    }

    // [列表]
    Expr::List(v)=> {
      Litr::List(Box::new(
        v.iter().map(|e| this.calc(e)).collect()
      ))
    }

    // Class {}创建实例
    Expr::NewInst(ins)=> {
      let cls = this.find_class(ins.cls);
      let mut v = vec![Litr::Uninit;cls.props.len()];
      let module = this.module;
      'a: for (id, e) in ins.val.0.iter() {
        for (n, prop) in cls.props.iter().enumerate() {
          if prop.name == *id {
            if !prop.public && cls.module != module {
              err(&format!("成员属性'{}'是私有的。",id))
            }
            v[n] = this.clone().calc(e);
            continue 'a;
          }
        }
        err(&format!("'{}'类型不存在'{}'属性。", cls.name, id.str()))
      }
      Litr::Inst(Box::new(Instance {cls, v:v.into()}))
    }

    // -.运算符
    Expr::ModFuncAcc(acc)=> {
      let modname = acc.0;
      let funcname = acc.1;
      let mods = unsafe {&(*this.module)};
      for def in mods.imports.iter() {
        if def.name == modname {
          for (id, func) in def.funcs.iter() {
            if *id == funcname {
              return Litr::Func(Box::new(func.clone()));
            }
          }
          err(&format!("模块'{}'中没有'{}'函数",modname,funcname))
        }
      }
      err(&format!("没有导入'{}'模块",modname))
    }

    Expr::ModClsAcc(_)=> err("类型声明不是一个值。考虑使用`class A = B`语句代替"),

    // 访问类方法
    Expr::ImplAccess(acc)=> {
      /// 在class中找一个函数
      fn find_fn(cls:&ClassDef, find:Interned, this_module:*mut Module)->Litr {
        for func in cls.statics.iter() {
          if func.name == find {
            if !func.public && cls.module != this_module {
              err(&format!("'{}'类型的静态方法'{}'是私有的。", cls.name, find))
            }
            return Litr::Func(Box::new(Function::Local(Box::new(func.f.clone()))));
          }
        }
        for func in cls.methods.iter() {
          if !func.public && cls.module != this_module {
            err(&format!("'{}'类型中的方法'{}'是私有的。", cls.name, find))
          }
          if func.name == find {
            return Litr::Func(Box::new(Function::Local(Box::new(func.f.clone()))));
          }
        }
        err(&format!("'{}'类型没有'{}'静态方法", cls.name, find.str()));
      }

      let find = acc.1;
      if let Expr::Variant(id) = acc.0 {
        let cls = this.find_class(id);
        return find_fn(cls, find, this.module);
      }

      if let Expr::ModClsAcc(acc) = &acc.0 {
        let modname = acc.0;
        let clsname = acc.1;
        let mods = unsafe {&(*this.module)};
        for def in mods.imports.iter() {
          if def.name == modname {
            for (name, cls) in def.classes.iter() {
              if *name == clsname {
                return find_fn(unsafe{&**cls}, find, this.module)
              }
            }
            err(&format!("模块'{}'中没有'{}'类型",modname,clsname))
          }
        }
        err(&format!("没有导入'{}'模块",modname))
      }
      err("::左侧必须是个类型")
    }

    Expr::Property(acc)=> {
      let find = acc.1;
      match acc.0 {
        Expr::Variant(id)=> {
          let from = unsafe{&mut *(this.var(id) as *mut Litr)};
          get_prop(this, from, find).own()
        }
        _=> {
          let scope = *this;
          let from = &mut this.calc(&acc.0);
          get_prop(this, from, find).own()
        }
      }
    }

    Expr::Kself => unsafe{(*this.kself).clone()},

    Expr::Empty => err("得到空表达式"),
    _=> err("未实装的表达式 ")
  }
}

/// calc_ref既可能得到引用，也可能得到计算过的值
pub enum CalcRef {
  Ref(*mut Litr),
  Own(Box<Litr>)
}
impl CalcRef {
  /// 消耗CalcRef返回内部值
  pub fn own(self)-> Litr {
    match self {
      CalcRef::Ref(p)=> unsafe {(*p).clone()}
      CalcRef::Own(v)=> *v
    }
  }
}
impl std::ops::Deref for CalcRef {
  type Target = Litr;
  fn deref(&self) -> &Self::Target {
    match self {
      CalcRef::Ref(p)=> unsafe{&**p},
      CalcRef::Own(b)=> &**b
    }
  }
}
impl std::ops::DerefMut for CalcRef {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      CalcRef::Ref(p)=> unsafe{&mut **p},
      CalcRef::Own(b)=> &mut **b
    }
  }
}

/// 能引用优先引用的calc，能避免很多复制同时保证引用正确
pub fn calc_ref(this:&mut Scope, e:&Expr)-> CalcRef {
  match e {
    Expr::Kself=> {
      let v = unsafe{&mut *this.kself};
      CalcRef::Ref(v)
    }
    Expr::Property(acc)=> {
      let find = acc.1;
      let mut from = calc_ref(this, &acc.0);
      get_prop(this, &mut *from, find)
    }
    Expr::Variant(id)=> CalcRef::Ref(this.var(*id)),
    // todo: Expr::Index
    _=> {
      let v = this.calc(e);
      CalcRef::Own(Box::new(v))
    }
  }
}

/// 同calc_ref, 但额外返回一个ref处的scope指针
fn calc_ref_with_scope(this:&mut Scope, e:&Expr)-> (CalcRef, Scope) {
  match e {
    Expr::Kself=> {
      let v = CalcRef::Ref(unsafe{&mut *this.kself});
      let mut scope = *this;
      let kself = this.kself;
      while let Some(prt) = scope.parent {
        // 如果self是顶级作用域的self就返回顶级作用域
        if prt.kself == kself {
          return (v, prt);
        }
        scope = prt;
      }
      (v, *this)
    }
    Expr::Property(acc)=> {
      let find = acc.1;
      let (mut from, scope) = calc_ref_with_scope(this, &acc.0);
      (get_prop(this, &mut *from, find), scope)
    }
    Expr::Variant(id)=> {
      let (rf, scope) = this.var_with_scope(*id);
      (CalcRef::Ref(rf), scope)
    }
    // todo: Expr::Index
    _=> {
      let v = this.calc(e);
      // 如果是需要计算的量，就代表其作用域就在this
      (CalcRef::Own(Box::new(v)), *this)
    }
  }
}


/// 在作用域中从Litr中找.运算符指向的东西
fn get_prop(this:&Scope, from:&mut Litr, find:Interned)-> CalcRef {
  match from {
    Litr::Inst(inst)=> {
      let cannot_access_private = unsafe {(*inst.cls).module} != this.module;
      let cls = unsafe {&*inst.cls};

      // 先找属性
      let props = &cls.props;
      for (n, prop) in props.iter().enumerate() {
        if prop.name == find {
          if !prop.public && cannot_access_private {
            err(&format!("'{}'类型的成员属性'{}'是私有的", cls.name, find))
          }
          return CalcRef::Ref(&mut inst.v[n]);
        }
      }

      // 再找方法
      let methods = &cls.methods;
      for mthd in methods.iter() {
        if mthd.name == find {
          if !mthd.public && cannot_access_private {
            err(&format!("'{}'类型的成员方法'{}'是私有的", cls.name, find))
          }
          // 为函数绑定self
          let mut f = mthd.f.clone();
          f.bound = Some(from);
          let f = Litr::Func(Box::new(Function::Local(Box::new(f))));
          return CalcRef::Own(Box::new(f));
        }
      }

      err(&format!("'{}'类型上没有'{}'属性", cls.name, find))
    },
    _=> err("该类型属性还没实装")
  }
}


fn binary(this:&mut Scope, bin:&BinDecl)-> Litr {
  use Litr::*;
  let mut left = this.calc_ref(&bin.left);
  let right = this.calc_ref(&bin.right);
  /// 二元运算中普通数字的戏份
  macro_rules! impl_num {
    ($pan:literal $op:tt) => {{
      match (&*left, &*right) {
        (Int(l),Int(r))=> Int(l $op r),
        (Uint(l),Uint(r))=> Uint(l $op r),
        (Uint(l),Int(r))=> Uint(l $op *r as usize),
        (Float(l),Float(r))=> Float(l $op r),
        (Float(l),Int(r))=> Float(l $op *r as f64),
        _=> err($pan)
      }
    }};
    ($pan:literal $op:tt $n:tt)=> {{
      if match &*right {
        Int(r) => *r == 0,
        Uint(r) => *r == 0,
        Float(r) => *r == 0.0,
        _=> false
      } {err("除数必须非0")}
      impl_num!($pan $op)
    }};
  }

  /// 二元运算中无符号数的戏份
  macro_rules! impl_unsigned {
    ($pan:literal $op:tt) => {{
      match (&*left, &*right) {
        (Uint(l), Uint(r))=> Uint(l $op r),
        (Uint(l), Int(r))=> Uint(l $op *r as usize),
        _=> err($pan)
      }
    }};
  }

  /// 数字修改并赋值
  macro_rules! impl_num_assign {
    ($o:tt) => {{
      // 将Int自动转为对应类型
      let n = match (&*left, &*right) {
        (Uint(l), Uint(r))=> Uint(l $o r),
        (Uint(l), Int(r))=> Uint(l $o *r as usize),
        (Int(l), Int(r))=> Int(l $o r),
        (Float(l), Float(r))=> Float(l $o r),
        (Float(l), Int(r))=> Float(l $o *r as f64),
        _=> err("运算并赋值的左右类型不同")
      };
      *left = n;
      Uninit
    }};
  }

  // 无符号数修改并赋值
  macro_rules! impl_unsigned_assign {
    ($op:tt) => {{
      // 数字默认为Int，所以所有数字类型安置Int自动转换
      let n = match (&*left, &*right) {
        (Uint(l), Uint(r))=> Uint(l $op r),
        (Uint(l), Int(r))=> Uint(l $op *r as usize),
        _=> err("按位运算并赋值只允许无符号数")
      };
      *left = n;
      Uninit
    }};
  }

  /// 比大小宏
  /// 
  /// 需要读堆的数据类型都需要以引用进行比较，减少复制开销
  macro_rules! impl_ord {($o:tt) => {{
    fn match_basic(l:&Litr,r:&Litr)-> bool {
      match (l, r) {
        (Uninit, Uninit)=> 0 $o 0,
        (Uint(l),Uint(r))=> l $o r,
        (Uint(l),Int(r))=> l $o &(*r as usize),
        (Uint(l),Float(r))=> l $o &(*r as usize),
        (Int(l), Uint(r))=> l $o &(*r as isize),
        (Int(l), Int(r))=> l $o r,
        (Int(l), Float(r))=> l $o &(*r as isize),
        (Float(l), Uint(r))=> l $o &(*r as f64),
        (Float(l), Int(r))=> l $o &(*r as f64),
        (Float(l), Float(r))=> l $o r,
        (Bool(l), Bool(r))=> l $o r,
        (Str(l), Str(r))=> l $o r,
        (Buffer(l), Buffer(r))=> l $o r,
        (List(l), List(r))=> match_list(l,r),
        (Obj, Obj)=> todo!("obj比较未实装"),
        (Inst(l),Inst(r))=> {
          if l.cls != r.cls {
            err("实例类型不同无法比较");
          }
          match_list(&*l.v, &*r.v)
        },
        _=> err("比较两侧类型不同。")
      }
    }

    fn match_list(l:&[Litr], r:&[Litr])-> bool {
      let len = l.len();
      if len != r.len() {
        err("列表长度不同，无法比较");
      }
      for i in 0..len {
        if !match_basic(&l[i],&r[i]) {
          return false
        };
      }
      true
    }

    Bool(match_basic(&*left,&*right))
  }}}

  /// 逻辑符
  macro_rules! impl_logic {
    ($o:tt) => {{
      match (&*left, &*right) {
        (Bool(l), Bool(r))=> Bool(*l $o *r),
        (Bool(l), Uninit)=> Bool(*l $o false),
        (Uninit, Bool(r))=> Bool(false $o *r),
        _=> err("逻辑运算符两边必须都为Bool")
      }
    }};
  }

  match &*bin.op {
    // 数字
    b"+" => {
      if let Str(l) = &*left {
        // litr.str()方法会把内部String复制一遍
        // 直接使用原String的引用可以避免这次复制
        if let Str(r) = &*right {
          let mut s = Box::new([l.as_str(),r.as_str()].concat());
          return Str(s);
        }
        let r = right.str();
        let mut s = Box::new([l.as_str(),r.as_str()].concat());
        return Str(s);
      }
      impl_num!("相加类型不同" +)
    },
    b"-" => impl_num!("相减类型不同" -),
    b"*" => impl_num!("相乘类型不同" *),
    b"%" => impl_num!("求余类型不同" %),
    b"/" => impl_num!("相除类型不同" / 0),

    // unsigned
    b"<<" => impl_unsigned!("左移需要左值无符号" <<),
    b">>" => impl_unsigned!("右移需要左值无符号" >>),
    b"&" => impl_unsigned!("&需要左值无符号" &),
    b"^" => impl_unsigned!("^需要左值无符号" ^),
    b"|" => impl_unsigned!("|需要左值无符号" |),

    // 赋值
    b"=" => {
      /// 如果值包含本地函数就为函数定义处增加一层引用计数
      fn may_add_ref(v:&Litr, target_scope: Scope) {
        match v {
          Litr::Func(f)=> {
            if let Function::Local(f) = &**f {
              outlive::outlive_to((**f).clone(),target_scope);
            }
          }
          Litr::List(l)=> 
            l.iter().for_each(|item|may_add_ref(item, target_scope)),
          Litr::Inst(inst)=> 
            inst.v.iter().for_each(|item|may_add_ref(item, target_scope)),
          Litr::Obj=> {err("obj赋值未实装")}
          _=> {}
        }
      }

      let (mut left, scope) = calc_ref_with_scope(this, &bin.left);
      let right = right.own();
      may_add_ref(&right, scope);
      println!("{this:?} {scope:?}");
      *left = right;
      Uninit
    }
    b"+=" => impl_num_assign!(+),
    b"-=" => impl_num_assign!(-),
    b"*=" => impl_num_assign!(*),
    b"/=" => impl_num_assign!(/),
    b"%=" => impl_num_assign!(%),

    b"&=" => impl_unsigned_assign!(&),
    b"^=" => impl_unsigned_assign!(^),
    b"|=" => impl_unsigned_assign!(|),
    b"<<=" => impl_unsigned_assign!(<<),
    b">>=" => impl_unsigned_assign!(>>),

    // 比较
    b"==" => impl_ord!(==),
    b"!=" => impl_ord!(!=),
    b">=" => impl_ord!(>=),
    b"<=" => impl_ord!(<=),
    b">" => impl_ord!(>),
    b"<" => impl_ord!(<),

    // 逻辑
    b"&&" => impl_logic!(&&),
    b"||" => impl_logic!(||),

    _=> err(&format!("未知运算符'{}'", String::from_utf8_lossy(&bin.op)))
  }
}

