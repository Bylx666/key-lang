//! 注释都在mod.rs里，这没有注解

use super::*;

/// 解析一个表达式，对应Expr
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
      let right = this.calc(&una.right);
      match una.op {
        b'-'=> {
          match right {
            Int(n)=> Int(-n),
            Float(n)=> Float(-n),
            _=> err("负号只能用在有符号数")
          }
        }
        b'!'=> {
          match right {
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
        err(&format!("该类型不存在'{}'属性。", id.str()))
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
              err(&format!("静态方法'{}'是私有的。",find))
            }
            return Litr::Func(Box::new(Function::Static(Box::new((cls.module, func.f.clone())))));
          }
        }
        for func in cls.methods.iter() {
          if !func.public && cls.module != this_module {
            err(&format!("方法'{}'是私有的。",find))
          }
          if func.name == find {
            return Litr::Func(Box::new(Function::Method(Box::new((cls, func.f.clone())))));
          }
        }
        err(&format!("该类型没有'{}'静态方法", find.str()));
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

    Expr::Empty => err("得到空表达式"),
    _=> err("未实装的表达式 ")
  }
}

fn binary(this:&mut Scope, bin:&BinDecl)-> Litr {
  use Litr::*;
  /// 二元运算中普通数字的戏份
  macro_rules! impl_num {
    ($pan:literal $op:tt) => {{
      let left = this.calc(&bin.left);
      let right = this.calc(&bin.right);
      impl_num!($pan,left,right $op)
    }};
    ($pan:literal $op:tt $n:tt)=> {{
      let left = this.calc(&bin.left);
      let right = this.calc(&bin.right);
      if match right {
        Int(r) => r == 0,
        Uint(r) => r == 0,
        Float(r) => r == 0.0,
        _=> false
      } {err("除数必须非0")}
      impl_num!($pan,left,right $op)
    }};
    ($pan:literal,$l:ident,$r:ident $op:tt) => {{
      match ($l.clone(), $r.clone()) {
        (Int(l),Int(r))=> Int(l $op r),
        (Uint(l),Uint(r))=> Uint(l $op r),
        (Uint(l),Int(r))=> Uint(l $op r as usize),
        (Float(l),Float(r))=> Float(l $op r),
        (Float(l),Int(r))=> Float(l $op r as f64),
        _=> err($pan)
      }
    }};
  }

  /// 二元运算中无符号数的戏份
  macro_rules! impl_unsigned {
    ($pan:literal $op:tt) => {{
      let left = this.calc(&bin.left);
      let right = this.calc(&bin.right);
      match (left, right) {
        (Uint(l), Uint(r))=> Uint(l $op r),
        (Uint(l), Int(r))=> Uint(l $op r as usize),
        _=> err($pan)
      }
    }};
  }

  /// 数字修改并赋值
  macro_rules! impl_num_assign {
    ($o:tt) => {{
      let left = this.calc(&bin.left);
      let right = this.calc(&bin.right);
      if let Expr::Variant(id) = bin.left {
        // 将Int自动转为对应类型
        let n = match (left, right) {
          (Uint(l), Uint(r))=> Uint(l $o r),
          (Uint(l), Int(r))=> Uint(l $o r as usize),
          (Int(l), Int(r))=> Int(l $o r),
          (Float(l), Float(r))=> Float(l $o r),
          (Float(l), Int(r))=> Float(l $o r as f64),
          _=> err("运算并赋值的左右类型不同")
        };
        *this.var(id) = n;
        return Uninit;
      }
      err("只能为变量赋值。");
    }};
  }

  // 无符号数修改并赋值
  macro_rules! impl_unsigned_assign {
    ($op:tt) => {{
      let left = this.calc(&bin.left);
      let right = this.calc(&bin.right);
      if let Expr::Variant(id) = bin.left {
        // 数字默认为Int，所以所有数字类型安置Int自动转换
        let n = match (left, right) {
          (Uint(l), Uint(r))=> Uint(l $op r),
          (Uint(l), Int(r))=> Uint(l $op r as usize),
          _=> err("按位运算并赋值只允许无符号数")
        };
        *this.var(id) = n;
        return Uninit;
      }
      err("只能为变量赋值。");
    }};
  }

  /// 比大小宏
  /// 
  /// 需要读堆的数据类型都需要以引用进行比较，减少复制开销
  macro_rules! impl_ord {($o:tt) => {{
    fn match_basic(l:&Litr,r:&Litr)-> bool {
      // 对于简单数字，复制开销并不大
      match (l.clone(), r.clone()) {
        (Uint(l),Uint(r))=> l $o r,
        (Uint(l),Int(r))=> l $o r as usize,
        (Uint(l),Float(r))=> l $o r as usize,
        (Int(l), Uint(r))=> l $o r as isize,
        (Int(l), Int(r))=> l $o r,
        (Int(l), Float(r))=> l $o r as isize,
        (Float(l), Uint(r))=> l $o r as f64,
        (Float(l), Int(r))=> l $o r as f64,
        (Float(l), Float(r))=> l $o r,
        (Bool(l), Bool(r))=> l $o r,
        _=> err("比较两侧类型不同。")
      }
    }

    // mayclone会在复制时拿到复制值的所有权
    let mut l_mayclone = Litr::Uninit;
    let mut l = match &bin.left {
      Expr::Variant(id)=> unsafe{&*(this.var(*id) as *mut Litr)}
      Expr::Literal(l)=> l,
      _=> {
        l_mayclone = this.calc(&bin.left);
        &l_mayclone
      }
    };
    let mut r_mayclone = Litr::Uninit;
    let mut r = match &bin.right {
      Expr::Variant(id)=> unsafe{&*(this.var(*id) as *mut Litr)}
      Expr::Literal(l)=> l,
      _=> {
        r_mayclone = this.calc(&bin.right);
        &r_mayclone
      }
    };
    Bool(match (l, r) {
      (Str(l), Str(r))=> l $o r,
      (List(l), List(r))=> {
        let len = l.len();
        if len != r.len() {
          err("列表长度不同，无法比较");
        }
        let mut b = true;
        for i in 0..len {
          if !match_basic(&l[i],&r[i]) {
            b = false;
            break;
          };
        }
        b
      },
      (Buffer(l), Buffer(r))=> l $o r,
      _=> match_basic(l,r)
    })
  }}}

  /// 逻辑符
  macro_rules! impl_logic {
    ($o:tt) => {{
      let mut left = this.calc(&bin.left);
      let mut right = this.calc(&bin.right);
      // 先把uninit同化成false
      if let Uninit = left {
        left = Bool(false)
      }
      if let Uninit = right {
        right = Bool(false)
      }

      match (left, right) {
        (Bool(l), Bool(r))=> Bool(l $o r),
        _=> err("逻辑运算符两边必须都为Bool")
      }
    }};
  }

  match &*bin.op {
    // 数字
    b"+" => {
      // 尽可能使用字符引用，避免复制字符(calc函数必定复制)
      let mut left_mayclone = Litr::Uninit;
      let left = match &bin.left {
        Expr::Variant(id)=> unsafe{&*(this.var(*id) as *mut Litr)},
        Expr::Literal(l)=> l,
        _=> {
          left_mayclone = this.calc(&bin.left);
          &left_mayclone
        }
      };
      let mut right_mayclone = Litr::Uninit;
      let right = match &bin.right {
        Expr::Variant(id)=> &*this.var(*id),
        Expr::Literal(l)=> l,
        _=> {
          right_mayclone = this.calc(&bin.right);
          &right_mayclone
        }
      };
      if let Str(l) = left {
        // litr.str()方法会把内部String复制一遍
        // 直接使用原String的引用可以避免这次复制
        if let Str(r) = right {
          let mut s = Box::new([l.as_str(),r.as_str()].concat());
          return Str(s);
        }
        let r = right.str();
        let mut s = Box::new([l.as_str(),r.as_str()].concat());
        return Str(s);
      }
      impl_num!("相加类型不同",left,right +)
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
      let (left,target_scope) = match bin.left {
        Expr::Variant(id)=> {
          let v = this.var_with_scope(id);
          (unsafe{&mut *(v.0 as *mut Litr)}, v.1)
        },
        _=> return Uninit
      };
      let right = this.calc(&bin.right);
      // 为函数定义处增加一层引用计数
      match &right {
        Litr::Func(f)=> {
          if let Function::Local(f) = &**f {
            outlive::outlive_to((**f).clone(),target_scope);
          }
        }
        Litr::List(l)=> {
          l.iter().for_each(|f|if let Litr::Func(f) = f {
            if let Function::Local(f) = &**f {
              outlive::outlive_to((**f).clone(),target_scope);
            }
          })
        }
        _=> {
          // todo!("Obj和Struct仍未实装");
        }
      }
      *left = right;
      return Uninit;
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