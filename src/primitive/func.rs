use crate::{
  intern::{intern, Interned},
  native::NativeFn,
  primitive::litr::{ArgDecl, Function, KsType, Litr, LocalFunc, LocalFuncRaw},
  runtime::{calc::CalcRef, Scope},
  scan::{expr::Expr, stmt::Statements},
};

use super::litr::LocalFuncRawArg;

pub fn method(f: &Function, name: Interned, cx: Scope, args: Vec<CalcRef>) -> Litr {
  match name.vec() {
    b"call" => kcall(f, args, cx),
    b"clone_here" => clone_here(f, args, cx),
    b"call_here" => call_here(f, args, cx),
    b"clone_top" => clone_top(f, args, cx),
    b"unzip" => unzip(f, cx),
    _ => panic!("func没有{}方法", name),
  }
}

/// 传入self并调用
pub fn kcall(f: &Function, mut args: Vec<CalcRef>, cx: Scope) -> Litr {
  assert!(args.len() >= 1, "func.call必须传入一个值作为self");
  let trans_args = args.split_off(1);
  let mut kself = args.pop().unwrap();
  match f {
    Function::Local(f) => Scope::call_local_with_self(
      f,
      trans_args.into_iter().map(|v| v.own()).collect(),
      &mut *kself,
    ),
    // 如果不是local就正常调用
    _ => cx.call(trans_args, f),
  }
}

/// 复制一个函数,但上下文在当前作用域
pub fn clone_here(f: &Function, _args: Vec<CalcRef>, cx: Scope) -> Litr {
  Litr::Func(match f {
    Function::Local(f) => Function::Local(LocalFunc::new(f.ptr, cx)),
    // 如果不是local就正常调用
    _ => f.clone(),
  })
}

/// 复制一个函数,但上下文在当前作用域
pub fn call_here(f: &Function, mut args: Vec<CalcRef>, cx: Scope) -> Litr {
  assert!(args.len() >= 1, "func.call_here必须传入一个值作为self");
  let trans_args = args.split_off(1);
  let mut kself = args.pop().unwrap();
  match f {
    Function::Local(f) => Scope::call_local_with_self(
      &LocalFunc::new(f.ptr, cx),
      trans_args.into_iter().map(|v| v.own()).collect(),
      &mut *kself,
    ),
    // 如果不是local就正常调用
    _ => cx.call(trans_args, f),
  }
}

/// 复制一个函数,但上下文在该模块的顶级作用域
pub fn clone_top(f: &Function, _args: Vec<CalcRef>, mut cx: Scope) -> Litr {
  // 获取顶级作用域
  while let Some(s) = &cx.parent {
    cx = s.clone()
  }
  Litr::Func(match f {
    Function::Local(f) => Function::Local(LocalFunc::new(f.ptr, cx)),
    // 如果不是local就正常调用
    _ => f.clone(),
  })
}

/// 忽略参数, 展开函数体
pub fn unzip(f: &Function, mut cx: Scope) -> Litr {
  let codes = match f {
    Function::Local(f) => &f.stmts,
    _ => panic!("unzip只能展开本地函数"),
  };

  // 暂时侵占该作用域的return_to
  let ori_return_to = cx.return_to;
  let mut unzip_return_to = Litr::Uninit;
  cx.return_to = &mut unzip_return_to;

  for (l, sm) in &codes.v {
    unsafe {
      crate::LINE = *l;
    }
    cx.evil(sm);

    // unzip过程中的return作为unzip返回值
    if cx.ended {
      // 让原作用域继续正常运行
      cx.ended = false;
      cx.return_to = ori_return_to;
      return std::mem::take(unsafe { &mut *cx.return_to });
    }
  }

  Litr::Uninit
}

pub fn statics() -> Vec<(Interned, NativeFn)> {
  vec![(intern(b"new"), s_new)]
}

fn s_new(mut s: Vec<CalcRef>, cx: Scope) -> Litr {
  let mut itr = s.iter_mut();
  let stmts = match itr.next() {
    Some(arg) => crate::scan::scan(match &**arg {
      Litr::Str(s) => s.as_bytes(),
      Litr::Buf(b) => b,
      _ => panic!("Func::new第一个参数必须是Str或Buf, 用来被解析为函数体"),
    }),
    None => Statements::default(),
  };

  let mut argdecl = Vec::new();
  while let Some(s) = itr.next() {
    let name = match &**s {
      Litr::Str(s) => intern(s.as_bytes()),
      _ => continue,
    };
    argdecl.push(ArgDecl {
      default: Expr::Literal(Litr::Uninit),
      name,
      t: KsType::Any,
    });
  }

  Litr::Func(Function::Local(LocalFunc::new(
    Box::into_raw(Box::new(LocalFuncRaw {
      argdecl: LocalFuncRawArg::Normal(argdecl),
      stmts,
      name: intern(b"unnamed"),
    })),
    cx,
  )))
}
