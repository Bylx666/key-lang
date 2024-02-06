//! 提供Ks数据和C交互的转换

use std::mem::transmute;
use std::slice::from_raw_parts as raw;
use crate::intern::Interned;
use crate::c::{dlopen,dlsym};


use crate::ast::{
  Litr, LocalFunc
};
use crate::runtime::ScopeInner;

static mut EXEC:Option<LocalFunc> = None;

/// 将ks函数传进extern函数的参数的实现
macro_rules! translate_local_impl {{
  $local:ident $(
    $n:literal $fname:ident($($arg:ident$(,)?)*) 
  )*
}=>{{
  let len = $local.argdecl.len();
  $(
    extern fn $fname($($arg:usize,)*)-> usize {
      let exec = unsafe {EXEC.as_mut().expect("未找到extern函数，这是bug")};
      let mut scope = exec.scope;
      let args = [$($arg,)*];
      let args = exec.argdecl.iter().enumerate()
        .map(|(i,_)| Litr::Uint(*args.get(i).unwrap_or(&0))).collect();
      let ret = scope.call_local(exec, args);
      match translate(ret) {
        Ok(v)=> v,
        Err(e)=> crate::runtime::err(&e)
      }
    }
  )*
  match len {
    $(
      $n => {
        unsafe {EXEC = Some($local.clone());}
        Ok($fname as usize)
      },
    )*
    _=> panic!("作为extern参数的函数不支持{}位参数",len)
  }
}}}

/// 将ks参数转为可与C交互的参数
pub fn translate(arg:Litr)-> Result<usize,String> {
  use Litr::*;
  match arg {
    Uninit=> Ok(0),
    Ref(p)=> todo!(),
    Bool(n)=> Ok(n as usize),
    Int(n)=> Ok(n as usize),
    Uint(n)=> Ok(n),
    Float(n)=> (unsafe{Ok(transmute(n))}),
    Str(p)=> Ok((*p).as_ptr() as usize),
    Buffer(v)=> {
      macro_rules! mat {($($t:ident)*)=>{{
        use crate::ast::Buf::*;
        match &*v {
          $(
            $t(v)=> Ok(v.as_ptr() as usize),
          )*
        }
      }}}

      mat!(U8 U16 U32 U64 I8 I16 I32 I64 F32 F64)
    }
    Func(p)=> {
      let exec = unsafe {&*p};
      use crate::ast::Executable::*;
      match exec {
        Local(f)=> translate_local_impl! { f 
          0  agent0 ()
          1  agent1 (a)
          2  agent2 (a,b)
          3  agent3 (a,b,c)
          4  agent4 (a,b,c,d)
          5  agent5 (a,b,c,d,e)
          6  agent6 (a,b,c,d,e,f)
          7  agent7 (a,b,c,d,e,f,g)
        },
        Extern(f)=> Ok(f.ptr),
        _=> Err("将运行时函数传进C函数是未定义行为".to_string())
      }
    }
    List(_)=> Err("列表类型无法作为C指针传递".to_string())
  }
}


use super::{ExternFunc, Scope, err};
pub fn call_extern(this:&Scope, f:&ExternFunc, args:Vec<Litr>)-> Litr {
  let len = f.argdecl.len();
  let mut args = args.into_iter();

  macro_rules! impl_arg {
    {$(
      $n:literal $($arg:ident)*
    )*} => {
      match len {
        $(
          $n => {
            let callable:extern fn($($arg:usize,)*)-> usize = unsafe {transmute(f.ptr)};
            let mut eargs = [0usize;$n];
            eargs.iter_mut().enumerate().for_each(|(i,p)| {
              if let Some(v) = args.next() {
                let transed = translate(v).unwrap_or_else(|e|err(&e));
                *p = transed
              }
            });
            let [$($arg,)*] = eargs;
            let ret = callable($($arg,)*);
            Litr::Uint(ret)
          }
        )*
        _=> {err(&format!("extern函数不支持{}位参数", len))}
      }
    }
  }
  impl_arg!{
    0
    1  a
    2  a b
    3  a b c
    4  a b c d
    5  a b c d e 
    6  a b c d e f 
    7  a b c d e f g
    8  a b c d e f g h
    9  a b c d e f g h i 
    10 a b c d e f g h i j
    11 a b c d e f g h i j k
    12 a b c d e f g h i j k l
    13 a b c d e f g h i j k l m
    14 a b c d e f g h i j k l m n
    15 a b c d e f g h i j k l m n o
  }
}

