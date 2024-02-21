//! 注释都在mod.rs里，这没有注解

use crate::native::BoundNativeMethod;

use super::*;

/// calc_ref既可能得到引用，也可能得到计算过的值
pub enum CalcRef {
  Ref(*mut Litr),
  Own(Litr)
}
impl CalcRef {
  /// 消耗CalcRef返回内部值
  pub fn own(self)-> Litr {
    match self {
      CalcRef::Ref(p)=> unsafe {(*p).clone()}
      CalcRef::Own(v)=> v
    }
  }
}
impl std::ops::Deref for CalcRef {
  type Target = Litr;
  fn deref(&self) -> &Self::Target {
    match self {
      CalcRef::Ref(p)=> unsafe{&**p},
      CalcRef::Own(b)=> b
    }
  }
}
impl std::ops::DerefMut for CalcRef {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      CalcRef::Ref(p)=> unsafe{&mut **p},
      CalcRef::Own(b)=> b
    }
  }
}

impl Scope {
  /// 解析一个表达式，对应Expr
  /// 
  /// 该函数必定发生复制
  pub fn calc(&mut self,e:&Expr)-> Litr {
    use Litr::*;
    match e {
      Expr::Call { args, targ }=> self.call(args, targ),

      Expr::Literal(litr)=> litr.clone(),

      Expr::Variant(id)=> self.var(*id).clone(),

      // 函数表达式
      Expr::LocalDecl(local)=> {
        let exec = LocalFunc::new(local, *self);
        Litr::Func(Function::Local(exec))
      }

      // 二元运算符
      Expr::Binary { left, right, op }=> binary(self, left, right, op),

      // 一元运算符
      Expr::Unary{right, op}=> {
        let right = self.calc_ref(right);
        match op {
          b'-'=> {
            match &*right {
              Int(n)=> Int(-n),
              Float(n)=> Float(-n),
              _=> err!("负号只能用在有符号数")
            }
          }
          b'!'=> {
            match &*right {
              Bool(b)=> Bool(!b),
              Int(n)=> Int(!n),
              Uint(n)=> Uint(!n),
              Uninit => Bool(true),
              _=> err!("!运算符只能用于整数和Bool")
            }
          }_=>Uninit
        }
      }

      // [列表]
      Expr::List(v)=> Litr::List(
        v.iter().map(|e| self.calc(e)).collect()
      ),

      // {a:"对",b:"象"}
      Expr::Obj(decl)=> {
        let mut map = HashMap::new();
        decl.iter().for_each(|(name, v)|{
          map.insert(*name, self.calc(v));
        });
        Litr::Obj(map)
      }

      // Class {}创建实例
      Expr::NewInst{cls: clsname, val}=> {
        let cls = self.find_class(*clsname);
        if let Class::Local(cls) = cls {
          let cls = unsafe {&*cls};
          let mut v = vec![Litr::Uninit;cls.props.len()];
          let module = self.exports;
          'a: for (id, e) in val.iter() {
            for (n, prop) in cls.props.iter().enumerate() {
              if prop.name == *id {
                if !prop.public && cls.module != module {
                  err!("成员属性'{}'是私有的。",id)
                }
                v[n] = self.clone().calc(e);
                continue 'a;
              }
            }
            err!("'{}'类型不存在'{}'属性。", cls.name, id.str())
          }
          Litr::Inst(Instance {cls, v:v.into()})
        }else {
          err!("无法直接构建原生类型'{}'", clsname)
        }
      }

      // -.运算符
      Expr::ModFuncAcc(modname, funcname)=> {
        let imports = unsafe {&*self.imports};
        for module in imports.iter() {
          match module {
            Module::Local(p)=> {
              let module = unsafe {&**p};
              if module.name == *modname {
                for (id, func) in module.funcs.iter() {
                  if *id == *funcname {
                    return Litr::Func(Function::Local(func.clone()));
                  }
                }
                err!("模块'{}'中没有'{}'函数",modname,funcname)
              }
            }
            Module::Native(p)=> {
              let module = unsafe {&**p};
              if module.name == *modname {
                for (id, func) in module.funcs.iter() {
                  if *id == *funcname {
                    return Litr::Func(Function::Native(func.clone()));
                  }
                }
                err!("模块'{}'中没有'{}'函数",modname,funcname)
              }
            }
          }
        }
        err!("没有导入'{}'模块",modname)
      }

      Expr::ModClsAcc(a,b)=> err!("类型声明不是一个值。考虑使用`class T = {}-:{}`语句代替",a b),

      // 访问类方法
      Expr::ImplAccess(e, find)=> {
        /// 在class中找一个函数
        fn find_fn(cls:Class, find:Interned, this_module:*mut LocalMod)->Litr {
          match cls {
            Class::Local(m)=> {
              let cls = unsafe {&*m};
              let cannot_access_private = cls.module != this_module;
              for func in cls.statics.iter() {
                if func.name == find {
                  if !func.public && cannot_access_private {
                    err!("'{}'类型的静态方法'{}'是私有的。", cls.name, find)
                  }
                  return Litr::Func(Function::Local(func.f.clone()));
                }
              }
              for func in cls.methods.iter() {
                if func.name == find {
                  if !func.public && cannot_access_private {
                    err!("'{}'类型中的方法'{}'是私有的。", cls.name, find)
                  }
                  return Litr::Func(Function::Local(func.f.clone()));
                }
              }
              err!("'{}'类型没有'{}'方法", cls.name, find.str());
            }
            Class::Native(m)=> {
              let cls = unsafe {&*m};
              for (name, func) in &cls.statics {
                if *name == find {
                  return Litr::Func(Function::Native(*func));
                }
              }
              err!("'{}'原生类型中没有'{}'静态方法", cls.name, find.str())
              // native模块的method使用bind太不安全了，只允许访问静态方法
            }
          }
        }

        if let Expr::Variant(id) = &**e {
          let cls = self.find_class(*id);
          return find_fn(cls, *find, self.exports);
        }

        if let Expr::ModClsAcc(s, modname) = &**e {
          let cls = self.find_class_in(*s, *modname);
          return find_fn(cls, *find, self.exports);
        }

        err!("::左侧必须是个类型")
      }

      Expr::Property(e, find)=> {
        match &**e {
          Expr::Variant(id)=> {
            let from = unsafe{&mut *(self.var(*id) as *mut Litr)};
            get_prop(self, from, *find).own()
          }
          _=> {
            let scope = *self;
            let from = &mut self.calc(&**e);
            get_prop(self, from, *find).own()
          }
        }
      }

      Expr::Kself => unsafe{(*self.kself).clone()},

      Expr::Empty => err!("得到空表达式"),
    }
  }

  /// 能引用优先引用的calc，能避免很多复制同时保证引用正确
  pub fn calc_ref(self:&mut Scope, e:&Expr)-> CalcRef {
    match e {
      Expr::Kself=> {
        let v = unsafe{&mut *self.kself};
        CalcRef::Ref(v)
      }
      Expr::Property(e, find)=> {
        let mut from = self.calc_ref(&e);
        get_prop(self, &mut *from, *find)
      }
      Expr::Variant(id)=> CalcRef::Ref(self.var(*id)),
      // todo: Expr::Index
      _=> {
        let v = self.calc(e);
        CalcRef::Own(v)
      }
    }
  }
}



/// 在一个作用域设置一个表达式为v
fn expr_set(this:&mut Scope, left:&Expr, right:Litr) {
  /// 寻找引用和引用本体所在的作用域
  fn calc_ref_with_scope(this: &mut Scope, e: &Expr)-> (CalcRef, Scope) {
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
      Expr::Property(e, find)=> {
        let (mut from, scope) = calc_ref_with_scope(this, &e);
        (get_prop(this, &mut *from, *find), scope)
      }
      Expr::Variant(id)=> {
        let (rf, scope) = this.var_with_scope(*id);
        (CalcRef::Ref(rf), scope)
      }
      // todo: Expr::Index
      _=> {
        let v = this.calc(e);
        // 如果是需要计算的量，就代表其作用域就在this
        (CalcRef::Own(v), *this)
      }
    }
  }
  use outlive::may_add_ref;

  // 如果是用到了setter的原生类实例就必须在此使用setter, 不能直接*left = right
  match left {
    Expr::Property(e, find)=> {
      let (mut left, scope) = calc_ref_with_scope(this, &e);
      may_add_ref(&right, scope);
      match &mut *left {
        Litr::Ninst(inst)=> {
          let cls = unsafe {&*inst.cls};
          (cls.setter)(inst, *find, right)
        }
        Litr::Obj(o)=> {
          o.insert(*find, right);
        },
        _=> *get_prop(this, &mut left, *find) = right
      }
    }
    // todo Expr::Index
    _=>{
      let (mut left, scope) = calc_ref_with_scope(this, left);
      may_add_ref(&right, scope);
      *left = right;
    }
  }
}


/// 在作用域中从Litr中找.运算符指向的东西
fn get_prop(this:&Scope, from:&mut Litr, find:Interned)-> CalcRef {
  match from {
    // 本地class的实例
    Litr::Inst(inst)=> {
      let cannot_access_private = unsafe {(*inst.cls).module} != this.exports;
      let cls = unsafe {&*inst.cls};

      // 先找属性
      let props = &cls.props;
      for (n, prop) in props.iter().enumerate() {
        if prop.name == find {
          if !prop.public && cannot_access_private {
            err!("'{}'类型的成员属性'{}'是私有的", cls.name, find)
          }
          return CalcRef::Ref(&mut inst.v[n]);
        }
      }

      // 再找方法
      let methods = &cls.methods;
      for mthd in methods.iter() {
        if mthd.name == find {
          if !mthd.public && cannot_access_private {
            err!("'{}'类型的成员方法'{}'是私有的", cls.name, find)
          }
          // 为函数绑定self
          let mut f = mthd.f.clone();
          f.bound = Some(from);
          let f = Litr::Func(Function::Local(f));
          return CalcRef::Own(f);
        }
      }

      err!("'{}'类型上没有'{}'属性", cls.name, find)
    },

    // 原生类的实例
    Litr::Ninst(inst)=> {
      let cls = unsafe {&*inst.cls};
      // 先找方法
      for (name, f) in cls.methods.iter() {
        if *name == find {
          return CalcRef::Own(Litr::Func(Function::NativeMethod(BoundNativeMethod {
            bind: inst,
            f: *f
          })));
        }
      }
      // 再找属性
      CalcRef::Own((cls.getter)(inst, find))
    }

    // 哈希表
    Litr::Obj(map)=> 
      CalcRef::Ref(map.get_mut(&find).unwrap_or(&mut Litr::Uninit)),

    Litr::Uninit=> err!("uninit没有属性"),
    _=> err!("该类型属性还没实装")
  }
}


fn binary(this:&mut Scope, left:&Box<Expr>, right:&Box<Expr>, op:&Box<[u8]>)-> Litr {
  use Litr::*;
  if &**op == b"=" {
    let v = this.calc(&right);
    expr_set(this, &left, v);
    return Uninit;
  }

  let mut left = this.calc_ref(&left);
  let right = this.calc_ref(&right);
  /// 二元运算中普通数字的戏份
  macro_rules! impl_num {
    ($pan:literal $op:tt) => {{
      match (&*left, &*right) {
        (Int(l),Int(r))=> Int(l $op r),
        (Uint(l),Uint(r))=> Uint(l $op r),
        (Uint(l),Int(r))=> Uint(l $op *r as usize),
        (Float(l),Float(r))=> Float(l $op r),
        (Float(l),Int(r))=> Float(l $op *r as f64),
        _=> err!($pan)
      }
    }};
    ($pan:literal $op:tt $n:tt)=> {{
      if match &*right {
        Int(r) => *r == 0,
        Uint(r) => *r == 0,
        Float(r) => *r == 0.0,
        _=> false
      } {err!("除数必须非0")}
      impl_num!($pan $op)
    }};
  }

  /// 二元运算中无符号数的戏份
  macro_rules! impl_unsigned {
    ($pan:literal $op:tt) => {{
      match (&*left, &*right) {
        (Uint(l), Uint(r))=> Uint(l $op r),
        (Uint(l), Int(r))=> Uint(l $op *r as usize),
        _=> err!($pan)
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
        _=> err!("运算并赋值的左右类型不同")
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
        _=> err!("按位运算并赋值只允许无符号数")
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
        (Obj(l), Obj(r))=> {
          if l.len() != r.len() {
            false
          }else {
            l.iter().all(|(k, left)| r.get(k).map_or(false, |right| match_basic(left, right)))
          }
        },
        (Inst(l),Inst(r))=> {
          if l.cls != r.cls {
            err!("实例类型不同无法比较");
          }
          match_list(&*l.v, &*r.v)
        },
        _=> false
      }
    }

    fn match_list(l:&[Litr], r:&[Litr])-> bool {
      let len = l.len();
      if len != r.len() {
        err!("列表长度不同，无法比较");
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
        _=> err!("{}两边必须都为Bool或uninit", stringify!($o))
      }
    }};
  }

  match &**op {
    // 数字
    b"+" => {
      if let Str(l) = &*left {
        // litr.str()方法会把内部String复制一遍
        // 直接使用原String的引用可以避免这次复制
        if let Str(r) = &*right {
          return Str([l.as_str(),r.as_str()].concat());
        }
        let r = right.str();
        return Str([l.as_str(),r.as_str()].concat());
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

    _=> err!("未知运算符'{}'", String::from_utf8_lossy(&op))
  }
}

