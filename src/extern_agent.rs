type HMOD = usize;
type FARPROC = usize;
type LPCS = *const u8;
extern {
  fn LoadLibraryA(src:LPCS)-> HMOD;
  fn GetProcAddress(lib:HMOD, name:LPCS)-> FARPROC;
}

use std::mem::transmute as trans;
use std::slice::from_raw_parts as raw;

use crate::intern::Interned;
pub struct Dll (usize);
impl Dll {
  /// 加载一个动态库
  pub fn load(s:&[u8])-> Self {
    unsafe {
      let lib = LoadLibraryA([s,&[0]].concat().as_ptr());
      if lib == 0 {
        panic!("无法找到动态库'{}'",String::from_utf8_lossy(s));
      }
      Dll(lib)
    }
  }
  /// 从动态库中寻找一个函数
  /// 
  /// 返回一个指针，需要自己transmute并检查非零
  pub unsafe fn get_func(&self, sym:&[u8])-> usize {
    let mut s = [sym,&[0]].concat();
    unsafe {
      GetProcAddress(self.0, s.as_ptr())
    }
  }
}


use crate::ast::{
  Litr, LocalFunc
};
use crate::runtime::Scope;

static mut SCOPE:Option<*mut Scope> = None;
static mut EXEC:Option<LocalFunc> = None;

/// 若ks参数中存在函数，则需要设置其作用域
pub fn set_scope(s:&mut Scope) {
  unsafe {SCOPE = Some(s as *mut Scope);}
}

/// 将ks函数传进extern函数的参数的实现
macro_rules! translate_local_impl {{
  $local:ident $(
    $n:literal $fname:ident($($arg:ident$(,)?)*) 
  )*
}=>{{
  let len = $local.args.len();
  $(
    extern fn $fname($($arg:usize,)*)-> usize {
      let scope = unsafe {&mut *SCOPE.expect("extern函数无作用域，这是bug")};
      let exec = unsafe {EXEC.as_ref().expect("未找到extern函数，这是bug")};
      let args = vec![$($arg,)*];
      let args = exec.args.iter().enumerate()
        .map(|(i,_)| Litr::Uint(*args.get(i).unwrap_or(&0))).collect();
      let ret = scope.call_local(exec, args);
      match translate(ret) {
        Ok(v)=> v,
        Err(e)=> scope.err(&e)
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
    Bool(n)=> Ok(n as usize),
    Int(n)=> Ok(n as usize),
    Uint(n)=> Ok(n),
    Float(n)=> (unsafe{Ok(trans(n))}),
    Str(p)=> Ok(unsafe{(*p).as_ptr() as usize}),
    Buffer(p)=> {
      let v = unsafe {&*p};
      macro_rules! mat {($($t:ident)*)=>{{
        use crate::ast::Buf::*;
        match v {
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
          8  agent8 (a,b,c,d,e,f,g,h)
          9  agent9 (a,b,c,d,e,f,g,h,i)
          10 agent10(a,b,c,d,e,f,g,h,i,j)
          11 agent11(a,b,c,d,e,f,g,h,i,j,k)
          12 agent12(a,b,c,d,e,f,g,h,i,j,k,l)
          13 agent13(a,b,c,d,e,f,g,h,i,j,k,l,m)
          14 agent14(a,b,c,d,e,f,g,h,i,j,k,l,m,n)
          15 agent15(a,b,c,d,e,f,g,h,i,j,k,l,m,n,o)
        },
        Extern(f)=> Ok(f.ptr),
        _=> Err("将运行时函数传进C函数是未定义行为".to_string())
      }
    }
    Variant(v)=> Err(format!("非法调用:试图将变量'{}'传入C函数",v.str())),
    Array(_)=> Err("列表类型无法作为C指针传递".to_string())
  }
  
}

