use crate::{
    intern::{intern, Interned},
    native::{NativeClassDef, NativeFn, NativeInstance},
    primitive::litr::Litr,
    runtime::{calc::CalcRef, Scope},
};

pub const ITER_END: usize = 1;

pub static mut SYMBOL_CLASS: *mut NativeClassDef = std::ptr::null_mut();

pub fn init() -> (Interned, *mut NativeClassDef) {
    unsafe {
        let s = super::new_static_class(b"Sym", vec![(intern(b"iter_end"), |_, _| iter_end())]);
        SYMBOL_CLASS = s.1;
        (*SYMBOL_CLASS).to_str = to_str;
        s
    }
}

pub fn is_sym(v: &NativeInstance) -> bool {
    unsafe { v.cls == SYMBOL_CLASS }
}
pub fn iter_end() -> Litr {
    unsafe {
        Litr::Ninst(NativeInstance {
            v: 1,
            w: 0,
            cls: SYMBOL_CLASS,
        })
    }
}

pub fn to_str(s: &NativeInstance) -> String {
    let t = match s.v {
        ITER_END => "迭代结束",
        _ => "未知",
    };
    format!("Sym {{ {} }}", t)
}
