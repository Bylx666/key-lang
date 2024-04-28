//! 创建一颗行星为你工作!
//!
//! 可阻塞可回调, 类似Promise

use self::litr::LocalFunc;
use super::*;
use std::sync::{Condvar, Mutex};

/// 可以调用fall的Planet类
pub static mut PLANET_CLASS: *mut NativeClassDef = std::ptr::null_mut();
/// 可以调用ok方法完成Planet的类
static mut PLANET_CALLER_CLASS: *mut NativeClassDef = std::ptr::null_mut();

#[derive(Debug)]
enum PlanetState {
    /// 异步未完成, write值不可用
    Scroll = 0,
    /// 异步已完成, write值可用
    Ok = 1,
    /// 值已被取走
    Died = 2,
}

#[derive(Debug)]
pub struct Planet {
    /// 异步状态
    state: PlanetState,
    /// 返回值的位置
    write: Litr,
    /// 调用过fall方法阻塞了主线程
    fallen: Option<(*mut Mutex<bool>, *const Condvar)>,
}
impl Planet {
    /// 取走异步结果的值
    fn take(&mut self) -> Litr {
        if let PlanetState::Died = self.state {
            panic!("该次Planet已完成, 无法重复取值")
        }
        self.state = PlanetState::Died;
        std::mem::take(&mut self.write)
    }
}

pub fn init() -> (Interned, *mut NativeClassDef) {
    unsafe {
        // 初始化Planet
        let s = new_static_class(
            b"Planet",
            vec![(intern(b"new"), new), (intern(b"all"), all)],
        );
        PLANET_CLASS = s.1;
        let planet = &mut *PLANET_CLASS;
        planet.methods.push((intern(b"fall"), fall));
        planet.methods.push((intern(b"then"), fall));
        planet.onclone = |_| panic!("无法复制行星!请尝试使用`take`函数.");
        planet.ondrop = |inst| unsafe { std::ptr::drop_in_place(inst.v as *mut Planet) };

        // 初始化Planet okay调用者的类
        PLANET_CALLER_CLASS = new_static_class(b"Planet.okay", vec![]).1;
        let caller = &mut *PLANET_CALLER_CLASS;
        caller.methods.push((intern(b"ok"), ok));
        s
    }
}

/// 参数筛选为单函数
#[inline]
fn to_func(args: &[CalcRef]) -> &Function {
    if let Some(f) = args.first() {
        match &**f {
            Litr::Func(f) => f,
            _ => panic!("第一个参数必须是函数"),
        }
    } else {
        panic!("第一个参数必须是函数")
    }
}

/// 让行星坠落阻塞主线程
fn fall(inst: &mut NativeInstance, args: Vec<CalcRef>, cx: Scope) -> Litr {
    let plan = unsafe { &mut *(inst.v as *mut Planet) };
    rust_fall(plan)
}

/// 完成行星任务并传回答案
fn ok(inst: &mut NativeInstance, args: Vec<CalcRef>, cx: Scope) -> Litr {
    let plan = unsafe { &mut *(inst.v as *mut Planet) };
    if !matches!(plan.state, PlanetState::Scroll) {
        return Litr::Uninit;
    }

    let okay = args.into_iter().next().map_or(Litr::Uninit, |n| n.own());
    rust_ok(plan, okay);

    Litr::Uninit
}

/// 创建一颗行星
fn new(args: Vec<CalcRef>, cx: Scope) -> Litr {
    let f = to_func(&args);
    let plan = rust_new();
    let caller = Litr::Ninst(NativeInstance {
        cls: unsafe { PLANET_CALLER_CLASS },
        v: plan as _,
        w: 0,
    });
    match f {
        Function::Local(f) => f.scope.call_local(f, vec![caller]),
        Function::Native(f) => f(vec![CalcRef::Own(caller)], cx),
        _ => panic!("无法使用extern函数作为Planet参数"),
    };
    Litr::Ninst(NativeInstance {
        v: plan as _,
        w: 0,
        cls: unsafe { PLANET_CLASS },
    })
}

/// 降落所有行星并返回参数长度的列表作为结果
fn all(args: Vec<CalcRef>, cx: Scope) -> Litr {
    let mut res = Vec::with_capacity(args.len());
    for arg in args {
        let plan = if let Litr::Ninst(inst) = &*arg {
            if inst.cls == unsafe { PLANET_CLASS } {
                unsafe { &mut *(inst.v as *mut Planet) }
            } else {
                panic!("Planet::all需要所有参数都是Planet")
            }
        } else {
            todo!("Planet::all需要所有参数都是Planet")
        };

        res.push(rust_fall(plan))
    }
    Litr::List(res)
}

// --原生--
pub fn rust_new() -> *mut Planet {
    Box::into_raw(Box::new(Planet {
        state: PlanetState::Scroll,
        write: Litr::Uninit,
        fallen: None,
    }))
}

fn rust_fall(plan: &mut Planet) -> Litr {
    if matches!(plan.state, PlanetState::Ok) {
        return plan.take();
    }

    // ok原则上只可被调用一次, 没有数据竞争问题
    let mut lock = Mutex::new(true);
    let cv = Condvar::new();
    plan.fallen = Some((&mut lock, &cv));
    let mut locked = lock.lock().unwrap();
    while *locked {
        locked = cv.wait(locked).unwrap();
    }
    plan.take()
}

pub fn rust_ok(plan: &mut Planet, okay: Litr) {
    plan.state = PlanetState::Ok;
    plan.write = okay;

    // 把阻塞死锁解开
    if let Some((lock, cv)) = plan.fallen {
        unsafe {
            *(*lock).lock().unwrap() = false;
            (*cv).notify_one();
        }
    }
}
