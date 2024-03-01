//! 注释都在mod.rs里，这没有注解

use crate::{native::{BoundNativeMethod, NaitveInstanceRef, NativeInstance}, primitive};

use super::*;

/// calc_ref既可能得到引用，也可能得到计算过的值
#[derive(Debug, Clone)]
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
  pub fn uninit()-> Self {
    CalcRef::Own(Litr::Uninit)
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
  pub fn calc(mut self,e:&Expr)-> Litr {
    match e {
      Expr::Call { args, targ }=> {
        // 先捕获方法调用
        // if let Expr::Property(left, right) = targ {
        //   self.call_
        // }
        let targ = self.calc_ref(targ);
        let args = args.iter().map(|v|self.calc_ref(v)).collect();
        self.call(args, targ)
      },

      Expr::Index { left, i }=> {
        let left = self.calc_ref(left);
        let i = self.calc_ref(i);
        index(left, i).own()
      },

      Expr::Literal(litr)=> litr.clone(),

      Expr::Variant(id)=> self.var(*id).unwrap_or_else(||err!("无法找到变量 '{}'", id.str())).own(),

      // 函数表达式
      Expr::LocalDecl(local)=> {
        let exec = LocalFunc::new(local, self);
        Litr::Func(Function::Local(exec))
      }

      // 二元运算符
      Expr::Binary { left, right, op }=> binary(self, left, right, op),

      // 一元运算符
      Expr::Unary{right, op}=> {
        use Litr::*;
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
      Expr::NewInst{cls, val}=> {
        let (cls, clsname) = match &**cls {
          Expr::ModClsAcc(modname, clsname)=> 
            (self.find_class_in(*modname, *clsname), clsname),
          Expr::Variant(clsname)=> 
            (self.find_class(*clsname).unwrap_or_else(||err!("未定义类 '{}'", clsname.str())), clsname),
          _=> err!("构建实例::左侧必须是类型名")
        };
        if let Class::Local(cls) = cls {
          let cls = unsafe {&mut *cls};
          let mut v = vec![Litr::Uninit;cls.props.len()];
          let cannot_access_private = self.exports != cls.module;
          'a: for (id, e) in val.iter() {
            for (n, prop) in cls.props.iter().enumerate() {
              if prop.name == *id {
                if !prop.public && cannot_access_private {
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
          err!("无法直接构建原生类型'{}'", clsname.str())
        }
      }

      // -.运算符
      Expr::ModFuncAcc(modname, funcname)=> {
        let imports = unsafe {&*self.imports};
        for (name, module) in imports.iter() {
          if name == modname {
            match module {
              Module::Local(m)=> {
                for (id, func) in unsafe{(**m).funcs.iter()} {
                  if *id == *funcname {
                    return Litr::Func(Function::Local(func.clone()));
                  }
                }
                err!("模块'{}'中没有'{}'函数",modname,funcname)
              }
              Module::Native(m)=> {
                for (id, func) in unsafe{(**m).funcs.iter()} {
                  if *id == *funcname {
                    return Litr::Func(Function::Native(func.clone()));
                  }
                }
                err!("原生模块'{}'中没有'{}'函数",modname,funcname)
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
          let cls = self.find_class(*id).unwrap_or_else(||err!("未定义类 '{}'", id.str()));
          return find_fn(cls, *find, self.exports);
        }

        if let Expr::ModClsAcc(s, modname) = &**e {
          let cls = self.find_class_in(*s, *modname);
          return find_fn(cls, *find, self.exports);
        }

        err!("::左侧必须是个类型")
      }

      Expr::Property(e, find)=> {
        let scope = self;
        let from = self.calc_ref(&**e);
        get_prop(self, from, *find)
      }

      Expr::Kself => unsafe{(*self.kself).clone()},

      // is操作符
      Expr::Is { left, right }=> {
        let v = self.calc_ref(left);
        let right = match &**right {
          Expr::Variant(id)=> id,
          Expr::ModClsAcc(modname, clsname)=> {
            let cls = self.find_class_in(*modname, *clsname);
            return Litr::Bool(match &*v {
              Litr::Inst(inst)=> match cls {
                Class::Local(cls)=> cls == inst.cls,
                _=> false
              }
              Litr::Ninst(inst)=> match cls {
                Class::Native(cls)=> cls == inst.cls,
                _=> false
              }
              _=> false
            })
          }
          _=> err!("is操作符右边必须是类型名")
        };
        macro_rules! matcher {($($d:ident)*)=> {
          match &*v {
            Litr::Inst(inst)=> Litr::Bool(match self.find_class(*right) {
              Some(Class::Local(c))=> c == inst.cls,
              _=> false
            }),
            Litr::Ninst(inst)=> Litr::Bool(match self.find_class(*right) {
              _=> false,
              Some(Class::Native(c))=> c == inst.cls
            }),
            Litr::Uninit=> Litr::Bool(false),
            $(
              Litr::$d(_) => Litr::Bool(intern(stringify!($d).as_bytes()) == *right),
            )*
          }
        }}
        matcher!{
          Bool Buf Float Func Int List Obj Str Sym Uint
        }
      }
      Expr::Empty => err!("得到空表达式"),
    }
  }

  /// 能引用优先引用的calc，能避免很多复制同时保证引用正确
  pub fn calc_ref(mut self, e:&Expr)-> CalcRef {
    match e {
      Expr::Kself=> {
        let v = unsafe{&mut *self.kself};
        CalcRef::Ref(v)
      }
      Expr::Index { left, i }=> {
        let left = self.calc_ref(left);
        let i = self.calc_ref(i);
        index(left, i)
      },
      Expr::Variant(id)=> self.var(*id).unwrap_or_else(||err!("无法找到变量 '{}'", id.str())),
      _=> {
        let v = self.calc(e);
        CalcRef::Own(v)
      }
    }
  }
}



/// 在一个作用域设置一个表达式为v
fn expr_set(mut this: Scope, left: &Expr, right: Litr) {
  /// 获取属性的引用
  pub fn get_prop_ref(this: Scope, mut from:CalcRef, find:&Interned)-> CalcRef {
    match &mut *from {
      Litr::Obj(map)=> CalcRef::Ref(map.get_mut(find).unwrap_or_else(||err!("Obj中没有'{}'", find))),
      Litr::Inst(inst)=> {
        let cls = unsafe {&*inst.cls};
        let cannot_access_private = unsafe {(*inst.cls).module} != this.exports;
        let props = &cls.props;
        for (n, prop) in props.iter().enumerate() {
          if prop.name == *find {
            if !prop.public && cannot_access_private {
              err!("'{}'类型的成员属性'{}'是私有的", cls.name, find)
            }
            return CalcRef::Ref(&mut inst.v[n]);
          }
        }
        err!("'{}'类型上没有'{}'属性", cls.name, find)
      }
      // native instance有自己的setter和getter,引用传递不了的
      _=> CalcRef::Own(Litr::Uninit)
    }
  }

  /// 寻找引用和引用本体所在的作用域
  fn get_ref_with_scope(mut this: Scope, e: &Expr)-> (CalcRef, Scope) {
    match e {
      Expr::Kself=> {
        let v = CalcRef::Ref(unsafe{&mut *this.kself});
        let mut scope = this;
        let kself = this.kself;
        while let Some(prt) = scope.parent {
          // 如果self是顶级作用域的self就返回顶级作用域
          if prt.kself == kself {
            return (v, prt);
          }
          scope = prt;
        }
        (v, this)
      }
      Expr::Property(e, find)=> {
        let (from, scope) = get_ref_with_scope(this, &e);
        let p = get_prop_ref(this, from, find);
        (p, scope)
      }
      Expr::Variant(id)=> {
        let (rf, scope) = this.var_with_scope(*id);
        (CalcRef::Ref(rf), scope)
      }
      Expr::Index{left, i}=> {
        let (mut left, scope) = get_ref_with_scope(this, &left);
        let i = this.calc_ref(i);
        (index(left, i), scope)
      }
      _=> {
        let v = this.calc(e);
        // 如果是需要计算的量，就代表其作用域就在this
        (CalcRef::Own(v), this)
      }
    }
  }
  use outlive::may_add_ref;

  match left {
    // 捕获native instance的setter
    Expr::Property(e, find)=> {
      let (mut left, scope) = get_ref_with_scope(this, &e);
      // 如果左值不是引用就没必要继续运行
      let left = match left {
        CalcRef::Ref(p)=> unsafe {&mut*p},
        _=> return
      };
      may_add_ref(&right, scope);
      match left {
        Litr::Ninst(inst)=> {
          let cls = unsafe {&*inst.cls};
          (cls.setter)(inst, *find, right)
        }
        Litr::Obj(o)=> {
          o.insert(*find, right);
        },
        _=> *get_prop_ref(this, CalcRef::Ref(left), find) = right
      }
    }

    // 捕获index_set
    Expr::Index{left,i}=> {
      let (mut left, scope) = get_ref_with_scope(this, &left);
      // 如果左值不是引用就没必要继续运行
      let left = match left {
        CalcRef::Ref(p)=> unsafe {&mut*p},
        _=> return
      };
      let i = this.calc_ref(i);
      may_add_ref(&right, scope);
      match left {
        Litr::Inst(inst)=> {
          let fname = intern(b"@index_set");
          let cls = unsafe{&mut *inst.cls};
          let opt = cls.methods.iter_mut().find(|v|v.name == fname);
          match opt {
            Some(f)=> {
              let f = &mut f.f;
              f.bound = Some(Box::new(CalcRef::Ref(left)));
              f.scope.call_local(f, vec![i.own(), right]);
            }
            None=> err!("为'{}'实例索引赋值需要定义`@index_set`方法", cls.name)
          }
        },
        Litr::Ninst(inst)=> {
          (unsafe{&*inst.cls}.index_set)(inst, i, right);
        },
        Litr::Obj(map)=> {
          if let Litr::Str(s) = &*i {
            map.insert(intern(s.as_bytes()), right);
          }else {err!("Obj索引必须是Str")}
        }
        _=> *index(CalcRef::Ref(left), i) = right
      }
    }

    _=>{
      let (mut left, scope) = get_ref_with_scope(this, left);
      may_add_ref(&right, scope);
      *left = right;
    }
  }
}


/// 在作用域中从Litr中找'.'属性运算符指向的东西
fn get_prop(this:Scope, mut from:CalcRef, find:Interned)-> Litr {
  match &mut *from {
    // 本地class的实例
    Litr::Inst(inst)=> {
      let cannot_access_private = unsafe {(*inst.cls).module} != this.exports;
      let cls = unsafe {&*inst.cls};

      // 寻找属性
      let props = &cls.props;
      for (n, prop) in props.iter().enumerate() {
        if prop.name == find {
          if !prop.public && cannot_access_private {
            err!("'{}'类型的成员属性'{}'是私有的", cls.name, find)
          }
          return inst.v[n].clone();
        }
      }

      err!("'{}'类型上没有'{}'属性", cls.name, find)
    },

    // 原生类的实例
    Litr::Ninst(inst)=> {
      let cls = unsafe {&*inst.cls};
      (cls.getter)(inst, find)
    }

    // 哈希表
    // 直接clone是防止Obj作为临时变量使map引用失效
    Litr::Obj(map)=> 
      map.get_mut(&find).unwrap_or(&mut Litr::Uninit).clone(),

    // 以下都是对基本类型的getter行为
    Litr::Bool(v)=> match find.vec() {
      b"opposite"=> Litr::Bool(!*v),
      _=> Litr::Uninit
    },

    Litr::Buf(v)=> match find.vec() {
      b"len"=> Litr::Uint(v.len()),
      b"ref"=> Litr::Uint(v.as_mut_ptr() as usize),
      b"capacity"=> Litr::Uint(v.capacity()),
      _=> Litr::Uninit
    },

    Litr::Func(f)=> if find.vec() == b"type" {
      match f {
        Function::Local(_)=> Litr::Str("local".to_owned()),
        Function::Extern(_)=> Litr::Str("extern".to_owned()),
        Function::Native(_)=> Litr::Str("native".to_owned()),
        Function::NativeMethod(_)=> unreachable!()
      }
    }else {Litr::Uninit}

    Litr::List(v)=> match find.vec() {
      b"len"=> Litr::Uint(v.len()),
      b"capacity"=> Litr::Uint(v.capacity()),
      _=> Litr::Uninit
    },

    Litr::Str(s)=> match find.vec() {
      b"len"=> Litr::Uint(s.len()),
      b"char_len"=> Litr::Uint(s.chars().count()),
      b"lines"=> Litr::Uint(s.lines().count()),
      b"capacity"=> Litr::Uint(s.capacity()),
      _=> Litr::Uninit
    },

    _=> Litr::Uninit
  }
}

fn index(mut left:CalcRef, i:CalcRef)-> CalcRef {
  // 先判断Obj
  if let Litr::Obj(map) = &mut *left {
    if let Litr::Str(s) = &*i {
      return match map.get_mut(&intern(s.as_bytes())) {
        Some(v)=> CalcRef::Ref(v),
        None=> CalcRef::uninit()
      };
    }
    err!("Obj的索引必须使用Str")
  }

  // 判断实例index_get
  if let Litr::Inst(inst) = &mut *left {
    let fname = intern(b"@index_get");
    let cls = unsafe{&mut *inst.cls};
    let opt = cls.methods.iter_mut().find(|v|v.name == fname);
    if let Some(f) = opt {
      let f = &mut f.f;
      f.bound = Some(Box::new(left));      
      return CalcRef::Own(f.scope.call_local(f, vec![i.own()]));
    }
    err!("读取'{}'实例索引需要定义`@index_get`方法", cls.name)
  }

  // 判断原生类实例
  if let Litr::Ninst(inst) = &mut *left {
    return CalcRef::Own(unsafe{((*inst.cls).index_get)(inst, i)});
  }

  // 把只会用到数字索引的放一起判断
  let i = match &*i {
    Litr::Uint(n)=> *n,
    Litr::Int(n)=> (*n) as usize,
    _=> err!("index必须是整数")
  };
  match &mut *left {
    Litr::Buf(v)=> {
      if i>=v.len() {return CalcRef::uninit()}
      CalcRef::Own(Litr::Uint(v[i] as usize))
    }
    Litr::List(v)=> {
      if i>=v.len() {return CalcRef::uninit()}
      CalcRef::Ref(&mut v[i])
    }
    Litr::Str(n)=> {
      match n.chars().nth(i) {
        Some(c)=> CalcRef::Own(Litr::Str(c.to_string())),
        None=> CalcRef::uninit()
      }
    }
    Litr::Uint(n)=> {
      if i>=64 {return CalcRef::Own(Litr::Bool(false));}
      CalcRef::Own(Litr::Bool((*n & (1<<i)) != 0))
    }
    _=> CalcRef::uninit()
  }
}

fn binary(mut this: Scope, left:&Box<Expr>, right:&Box<Expr>, op:&Box<[u8]>)-> Litr {
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
        (Buf(l), Buf(r))=> l $o r,
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
        (Sym(l), Sym(r))=> l $o r,
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
    b"%" => impl_num!("求余类型不同" % 0),
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

