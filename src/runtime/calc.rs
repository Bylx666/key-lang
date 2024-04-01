//! 注释都在mod.rs里，这没有注解

use crate::{
  native::NativeInstance, 
  primitive::{self, litr::*, get_prop}
};
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
  /// 拿走Calcref可变引用的所有权
  pub fn take(&mut self)-> Litr {
    let mut v = Self::uninit();
    std::mem::swap(&mut v, self);
    match v {
      CalcRef::Ref(p)=> unsafe {(*p).clone()},
      CalcRef::Own(v)=> v
    }
  }
  pub const fn uninit()-> Self {
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
        let targ_ = self.calc_ref(targ);
        let targ = match &*targ_ {
          Litr::Func(f)=> f,
          _=> {
            let s = match &**targ {
              Expr::Literal(n)=> n.str(),
              Expr::Variant(n)=> n.str(),
              _=> "".to_string()
            };
            panic!("{s}不是一个函数")
          }
        };
        let args = args.iter().map(|v|self.calc_ref(v)).collect();
        self.call(args, targ)
      },

      Expr::CallMethod { args, targ, name }=> {
        let targ = self.calc_ref(targ);
        let args = args.iter().map(|v|self.calc_ref(v)).collect();
        self.call_method(args, targ, *name)
      },

      Expr::Index { left, i }=> {
        let left = self.calc_ref(left);
        let i = self.calc_ref(i);
        get_index(left, i).own()
      },

      Expr::Literal(litr)=> litr.clone(),

      Expr::Variant(id)=> self.var(*id).unwrap_or_else(||panic!("无法找到变量 '{}'", id.str())).own(),

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
              _=> panic!("负号只能用在有符号数")
            }
          }
          b'!'=> {
            match &*right {
              Bool(b)=> Bool(!b),
              Int(n)=> Int(!n),
              Uint(n)=> Uint(!n),
              Uninit => Bool(true),
              _=> panic!("!运算符只能用于整数和Bool")
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

      // Class::{}创建实例
      Expr::NewInst{cls, val}=> {
        let (cls, clsname) = match &**cls {
          Expr::ModClsAcc(modname, clsname)=> 
            (self.find_class_in(*modname, *clsname), clsname),
          Expr::Variant(clsname)=> 
            (self.find_class(*clsname).unwrap_or_else(||panic!("未定义类 '{}'", clsname.str())), clsname),
          _=> panic!("构建实例::左侧必须是类型名")
        };
        if let Class::Local(cls) = cls {
          let cls = unsafe {&mut *cls};
          let mut v = vec![Litr::Uninit;cls.props.len()];
          /// 记录哪个属性没有写入
          let mut writen = vec![false; cls.props.len()];
          /// 确认你在模块内还是模块外
          let can_access_private = self.exports == cls.cx.exports;
          'a: for (id, e) in val.iter() {
            for (n, prop) in cls.props.iter().enumerate() {
              if prop.name == *id {
                assert!(prop.public || can_access_private,
                  "成员属性'{}'是私有的",id);
                // 类型检查
                let right = self.calc(e);
                assert!(prop.typ.is(&right, cls.cx), "'{}'属性要求{:?}类型, 但传入了{:?}", id, prop.typ, right);
                // 写入值
                unsafe{
                  *v.get_unchecked_mut(n) = right;
                  *writen.get_unchecked_mut(n) = true;
                }
                continue 'a;
              }
            }
            panic!("'{}'类型不存在'{}'属性", cls.name, id.str())
          }
          // 如果你在模块外, 就不能缺省属性
          if !can_access_private {
            let strs = writen.iter().enumerate().filter_map(|(n,b)|if !*b {
              Some(unsafe{cls.props.get_unchecked(n).name}.str())
            }else {None}).collect::<Vec<String>>();
            // 如果有一个属性没写就报错
            if strs.len() > 0 {
              panic!("正在创建'{}'类型, 但以下属性{}的值未定义", cls.name, strs.join(", "));
            }
          }
          Litr::Inst(Instance {cls, v:v.into()})
        }else {
          panic!("无法直接构建原生类型'{}'", clsname.str())
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
                panic!("模块'{}'中没有'{}'函数",modname,funcname)
              }
              Module::Native(m)=> {
                for (id, func) in unsafe{(**m).funcs.iter()} {
                  if *id == *funcname {
                    return Litr::Func(Function::Native(func.clone()));
                  }
                }
                panic!("原生模块'{}'中没有'{}'函数",modname,funcname)
              }
            }
          }
        }
        panic!("没有导入'{}'模块",modname)
      }

      Expr::ModClsAcc(a,b)=> panic!("类型声明不是一个值。考虑使用`class T = {}-:{}`语句代替",a, b),

      // 访问类方法
      Expr::ImplAccess(e, find)=> {
        /// 在class中找一个函数
        fn find_fn(cls:Class, find:Interned, this_module:*mut LocalMod)->Litr {
          match cls {
            Class::Local(m)=> {
              let cls = unsafe {&*m};
              let can_access_private = cls.cx.exports == this_module;
              for func in cls.statics.iter() {
                if func.name == find {
                  assert!(func.public || can_access_private, 
                    "'{}'类型的静态方法'{}'是私有的。", cls.name, find);
                  
                  let f = LocalFunc::new(&func.f, cls.cx);
                  return Litr::Func(Function::Local(f));
                }
              }
              for func in cls.methods.iter() {
                if func.name == find {
                  assert!(!func.public || can_access_private,
                    "'{}'类型中的方法'{}'是私有的。", cls.name, find);
                  
                  let f = LocalFunc::new(&func.f, cls.cx);
                  return Litr::Func(Function::Local(f));
                }
              }
              panic!("'{}'类型没有'{}'方法", cls.name, find.str());
            }
            Class::Native(m)=> {
              let cls = unsafe {&*m};
              for (name, func) in &cls.statics {
                if *name == find {
                  return Litr::Func(Function::Native(*func));
                }
              }
              panic!("'{}'原生类型中没有'{}'静态方法", cls.name, find.str())
              // native模块的method使用bind太不安全了，只允许访问静态方法
            }
          }
        }

        if let Expr::Variant(id) = &**e {
          let cls = self.find_class(*id).unwrap_or_else(||panic!("未定义类 '{}'", id.str()));
          return find_fn(cls, *find, self.exports);
        }

        if let Expr::ModClsAcc(s, modname) = &**e {
          let cls = self.find_class_in(*s, *modname);
          return find_fn(cls, *find, self.exports);
        }

        panic!("::左侧必须是个类型")
      }

      Expr::Property(e, find)=> {
        let scope = self;
        let from = self.calc_ref(&**e);
        get_prop(self, from, *find).own()
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
          _=> panic!("is操作符右边必须是类型名")
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
      
      Expr::Empty => panic!("得到空表达式"),
    }
  }

  /// 能引用优先引用的calc，能避免很多复制同时保证引用正确
  pub fn calc_ref(mut self, e:&Expr)-> CalcRef {
    match e {
      Expr::Kself=> {
        let v = unsafe{&mut *self.kself};
        CalcRef::Ref(v)
      }
      Expr::Property(left, name)=> {
        let left = self.calc_ref(left);
        get_prop(self, left, *name)
      }
      Expr::Index { left, i }=> {
        let left = self.calc_ref(left);
        let i = self.calc_ref(i);
        get_index(left, i)
      },
      Expr::Variant(id)=> self.var(*id).unwrap_or_else(||panic!("无法找到变量 '{}'", id.str())),
      _=> {
        let v = self.calc(e);
        CalcRef::Own(v)
      }
    }
  }
  
  /// 遇到locked的变量会报错版的 calc_ref
  pub fn calc_ref_unlocked(mut self, e:&Expr)-> CalcRef {
    match e {
      Expr::Kself=> {
        let v = unsafe{&mut *self.kself};
        CalcRef::Ref(v)
      }
      Expr::Property(left, name)=> {
        let left = self.calc_ref_unlocked(left);
        get_prop(self, left, *name)
      }
      Expr::Index { left, i }=> {
        let left = self.calc_ref_unlocked(left);
        let i = self.calc_ref(i);
        get_index(left, i)
      },
      Expr::Variant(id)=> {
        fn var_locked(inner: &mut ScopeInner, id:Interned)-> CalcRef {
          for Variant { name, v, locked } in inner.vars.iter_mut().rev() {
            if id == *name {
              if *locked {
                panic!("'{name}'已被锁定, 考虑用复制该变量来更改")
              }
              return CalcRef::Ref(v);
            }
          }

          if let Some(parent) = &mut inner.parent {
            return var_locked(parent, id);
          }
          panic!("无法找到变量 '{}'", id.str());
        }
        let inner = &mut (*self);
        var_locked(inner, *id)
      },
      _=> self.calc_ref(e)
    }
  }

}



/// 在一个作用域设置一个表达式为v
fn expr_set(mut this: Scope, left: &Expr, right: Litr) {
  match left {
    // 捕获native instance的setter
    Expr::Property(e, find)=> {
      // 如果左值不是引用就没必要继续运行
      let left = match this.calc_ref_unlocked(e) {
        CalcRef::Ref(p)=> unsafe {&mut*p},
        _=> return
      };
      match left {
        Litr::Ninst(inst)=> {
          let cls = unsafe {&*inst.cls};
          (cls.setter)(inst, *find, right)
        }
        Litr::Obj(o)=> {
          o.insert(*find, right);
        }
        Litr::Inst(inst)=> {
          let cls = unsafe {&*inst.cls};
          let can_access_private = unsafe {(*inst.cls).cx.exports} == this.exports;
          let props = &cls.props;
          for (n, prop) in props.iter().enumerate() {
            if prop.name == *find {
              assert!(prop.public || can_access_private,
                "'{}'类型的成员属性'{}'是私有的", cls.name, find);
              
              // 类型检查
              assert!(prop.typ.is(&right, cls.cx), "'{}'属性要求{:?}类型, 但传入了{:?}", find, prop.typ, right);
              // 写入值
              unsafe{*inst.v.get_unchecked_mut(n) = right;}
              return;
            }
          }
          panic!("'{}'类型上没有'{}'属性", cls.name, find)
        }
        _=> ()
      }
    }

    // 捕获index_set
    Expr::Index{left,i}=> {
      let left = this.calc_ref_unlocked(left);
      // 如果左值不是引用就没必要继续运行
      let left = match left {
        CalcRef::Ref(p)=> unsafe {&mut*p},
        _=> return
      };
      let i = this.calc_ref(i);
      match left {
        Litr::Inst(inst)=> {
          let fname = intern(b"@index_set");
          let cls = unsafe{&mut *inst.cls};
          let opt = cls.methods.iter().find(|v|v.name == fname);
          match opt {
            Some(f)=> {
              let f = LocalFunc::new(&f.f, cls.cx);
              Scope::call_local_with_self(&f, vec![i.own(), right], left);
            }
            None=> panic!("为'{}'实例索引赋值需要定义`@index_set`方法", cls.name)
          }
        },
        Litr::Ninst(inst)=> {
          (unsafe{&*inst.cls}.index_set)(inst, i, right);
        },
        Litr::Obj(map)=> {
          if let Litr::Str(s) = &*i {
            map.insert(intern(s.as_bytes()), right);
          }else {panic!("Obj索引必须是Str")}
        }
        _=> *get_index(CalcRef::Ref(left), i) = right
      }
    }

    _=>{
      let mut left = this.calc_ref_unlocked(left);
      *left = right;
    }
  }
}


/// 获取一个ks值索引处的值
fn get_index(mut left:CalcRef, i:CalcRef)-> CalcRef {
  // 先判断Obj
  if let Litr::Obj(map) = &mut *left {
    if let Litr::Str(s) = &*i {
      return match map.get_mut(&intern(s.as_bytes())) {
        Some(v)=> CalcRef::Ref(v),
        None=> CalcRef::uninit()
      };
    }
    panic!("Obj的索引必须使用Str")
  }

  // 判断实例index_get
  let left = &mut *left;
  if let Litr::Inst(inst) = left {
    let fname = intern(b"@index_get");
    let cls = unsafe{&mut *inst.cls};
    let opt = cls.methods.iter().find(|v|v.name == fname);
    if let Some(f) = opt {
      let f = LocalFunc::new(&f.f, cls.cx);
      return CalcRef::Own(Scope::call_local_with_self(&f, vec![i.own()], left));
    }
    panic!("读取'{}'实例索引需要定义`@index_get`方法", cls.name)
  }

  // 判断原生类实例
  if let Litr::Ninst(inst) = &mut *left {
    return CalcRef::Own(unsafe{((*inst.cls).index_get)(inst, i)});
  }

  // 把只会用到数字索引的放一起判断
  let i = match &*i {
    Litr::Uint(n)=> *n,
    Litr::Int(n)=> (*n) as usize,
    _=> panic!("index必须是整数")
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
        (Int(l),Uint(r))=> Int(l $op *r as isize),
        (Float(l),Float(r))=> Float(l $op r),
        (Float(l),Int(r))=> Float(l $op *r as f64),
        (Int(l),Float(r))=> Float(*l as f64 $op r),
        _=> panic!($pan)
      }
    }};
  }

  /// 二元运算中无符号数的戏份
  macro_rules! impl_unsigned {
    ($pan:literal $op:tt) => {{
      match (&*left, &*right) {
        (Uint(l), Uint(r))=> Uint(l $op r),
        (Uint(l), Int(r))=> Uint(l $op *r as usize),
        (Int(l), Uint(r))=> Uint((*l as usize) $op r),
        _=> panic!($pan)
      }
    }};
  }

  /// 数字修改并赋值
  macro_rules! impl_num_assign {
    ($o:tt) => {{
      // 将Int自动转为对应类型
      let n = match (&*left, &*right) {
        (Uint(l), Uint(r))=> Uint(l $o r),
        (Uint(l), Int(r))=> Uint(*l $o *r as usize),
        (Int(l), Int(r))=> Int(l $o r),
        (Float(l), Float(r))=> Float(*l $o r),
        (Float(l), Int(r))=> Float(*l $o *r as f64),
        (Int(l), Float(r))=> Float((*l as f64) $o *r),
        _=> panic!("运算并赋值的左右类型不同")
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
        (Uint(l), Int(r))=> Uint(*l $op *r as usize),
        (Int(l), Uint(r))=> Uint((*l as usize) $op r),
        _=> panic!("按位运算并赋值只允许Uint")
      };
      *left = n;
      Uninit
    }};
  }

  /// 逻辑符
  macro_rules! impl_logic {
    ($o:tt) => {{
      match (&*left, &*right) {
        (Bool(l), Bool(r))=> Bool(*l $o *r),
        (Bool(l), Uninit)=> Bool(*l $o false),
        (Uninit, Bool(r))=> Bool(false $o *r),
        _=> panic!("{}两边必须都为Bool或uninit", stringify!($o))
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
    // 本应该判断非0的,还是让rust帮我报错吧
    b"%" => impl_num!("求余类型不同" %),
    b"/" => impl_num!("相除类型不同" /),

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
    b"==" => Bool(&*left == &*right),
    b"!=" => Bool(&*left != &*right),
    b">=" => Bool(&*left >= &*right),
    b"<=" => Bool(&*left <= &*right),
    b">" => Bool(&*left > &*right),
    b"<" => Bool(&*left < &*right),

    // 逻辑
    b"&&" => impl_logic!(&&),
    b"||" => impl_logic!(||),

    _=> panic!("未知运算符'{}'", String::from_utf8_lossy(&op))
  }
}
