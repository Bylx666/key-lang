
use super::*;

pub fn method(v:&mut Vec<u8>, scope:Scope, name:Interned, args:Vec<CalcRef>)-> Litr {
  match name.vec() {
    b"push"=> push(v, args),
    b"push_front"=> push_front(v, args),
    b"dedup"=> dedup(v, args, scope),
    b"sort"=> sort(v, args, scope),
    b"for_each"=> for_each(v, args, scope),
    b"map"=> map(v, args, scope),
    b"pop"=> pop(v, args),
    b"pop_front"=> pop_front(v, args),
    b"rev"=> rev(v),
    b"filter"=> filter(v, args, scope),
    b"filter_clone"=> filter_clone(v, args, scope),
    b"copy_within"=> copy_within(v, args),
    // b"splice"=> splice(v, args),
    _=> panic!("Buf没有{}方法",name)
  }
}

const fn to_u8(v:&Litr)-> u8 {
  match v {
    Litr::Int(n)=> *n as u8,
    Litr::Uint(n)=> *n as u8,
    _=> 0
  }
}
const fn to_usize(v:&Litr)-> usize {
  match v {
    Litr::Int(n)=> *n as usize,
    Litr::Uint(n)=> *n,
    _=> 0
  }
}

/// 推进数组或者单数字
fn push(v:&mut Vec<u8>, mut args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter_mut();
  match &**next_arg!(args "'push'方法需要一个数字,列表或数组作为参数") {
    Litr::Buf(right)=> v.extend_from_slice(right),
    Litr::List(right)=> v.extend_from_slice(
      &right.iter().map(|litr|to_u8(litr)).collect::<Box<[u8]>>()),
    n=> v.push(to_u8(n))
  };
  Litr::Uninit
}

/// 像是js的unshift
fn push_front(v:&mut Vec<u8>, mut args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter_mut();
  match &**next_arg!(args "'push_front'方法需要一个数字,列表或数组作为参数") {
    Litr::Buf(right)=> *v = [&**right, v].concat(),
    Litr::List(right)=> *v = [
      &*right.iter().map(|litr|to_u8(litr)).collect::<Box<[u8]>>(), v].concat(),
    n=> v.insert(0, to_u8(n))
  };
  Litr::Uninit
}

/// 去重 建议和sort联用
fn dedup(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  // 如果传了第一个参数就用dedup_by
  if let Some(f) = args.get(0) {
    let f = match &**f {
      Litr::Func(f)=> f,
      _=> panic!("buf.dedup第一个参数只能传函数")
    };
    v.dedup_by(|a,b| match scope.call(vec![
      CalcRef::Own(Litr::Uint(*a as usize)), CalcRef::Own(Litr::Uint(*b as usize))
    ], f) {
      Litr::Bool(b)=> b,
      _=> false
    });
  }else {
    v.dedup();
  }
  Litr::Uninit
}

/// 排序
fn sort(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  // 如果传了第一个参数就用sort_by
  if let Some(f) = args.get(0) {
    let f = match &**f {
      Litr::Func(f)=> f,
      _=> panic!("buf.sort第一个参数只能传函数")
    };
    use std::cmp::Ordering;
    v.sort_unstable_by(|a,b| match scope.call(vec![
      CalcRef::Own(Litr::Uint(*a as usize)), CalcRef::Own(Litr::Uint(*b as usize))
    ], f) {
      Litr::Bool(b)=> match b {
        true=> Ordering::Greater,
        false=> Ordering::Less,
      },
      _=> Ordering::Greater
    });
  }else {
    v.sort_unstable();
  }
  Litr::Uninit
}

/// 循环调用
fn for_each(v:&mut Vec<u8>, mut args:Vec<CalcRef>, scope:Scope)-> Litr {
  let mut args = args.iter_mut();
  let f = match &**next_arg!(args "buf.foreach需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.foreach第一个参数只能传函数")
  };
  v.iter().for_each(|a| {scope.call(vec![
    CalcRef::Own(Litr::Uint(*a as usize))
  ], f);});
  Litr::Uninit
}

/// 映射重构新Buf
fn map(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match args.get(0) {
    Some(f)=> match &**f {
      Litr::Func(f)=> f,
      _=> panic!("buf.map第一个参数只能传函数")
    },
    None=> panic!("buf.map需要一个函数作为参数")
  };
  Litr::Buf(v.iter().map(|a| match scope.call(vec![
    CalcRef::Own(Litr::Uint(*a as usize))
  ], f) {
    Litr::Uint(n)=> n as u8,
    Litr::Int(n)=> n as u8,
    _=> 0
  }).collect())
}

/// 从末尾切去一个, 可传一个数字作为切去数量
fn pop(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  if let Some(arg0) = args.get(0) {
    let at = match &**arg0 {
      Litr::Uint(n)=> *n,
      Litr::Int(n)=> *n as usize,
      _=> panic!("buf.pop的参数必须为整数")
    };
    if at >= v.len() {
      panic!("分界线索引{at}大于数组长度{}", v.len());
    }

    Litr::Buf(v.split_off(v.len() - at))
  }else {
    match v.pop() {
      Some(n)=> Litr::Uint(n as usize),
      None=> Litr::Uninit
    }
  }
}

/// 从开头切去一个, 可传一个数字作为切掉数量
fn pop_front(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  if let Some(arg0) = args.get(0) {
    let at = match &**arg0 {
      Litr::Uint(n)=> *n,
      Litr::Int(n)=> *n as usize,
      _=> panic!("buf.pop_front的参数必须为整数")
    };
    if at >= v.len() {
      panic!("分界线索引{at}大于数组长度{}", v.len());
    }

    let mut part = v.split_off(at);
    std::mem::swap(v, &mut part);
    Litr::Buf(part)
  }else {
    if v.len()==0 {return Litr::Uninit;}
    Litr::Uint(v.remove(0) as usize)
  }
}

/// 反转Buf
fn rev(v:&mut Vec<u8>)-> Litr {
  v.reverse();
  Litr::Uninit
}

/// 将当前数组按函数过滤
fn filter(v:&mut Vec<u8>, mut args:Vec<CalcRef>, scope:Scope)-> Litr {
  let mut args = args.iter_mut();
  let f = match &**next_arg!(args "buf.filter需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.map第一个参数只能传函数")
  };
  v.retain(|a|match scope.call(
    vec![CalcRef::Own(Litr::Uint(*a as usize))], f
  ) {
    Litr::Bool(b)=> b,
    _=> false
  });
  Litr::Uninit
}

/// filter的复制版本
fn filter_clone(v:&mut Vec<u8>, mut args:Vec<CalcRef>, scope:Scope)-> Litr {
  let mut args = args.iter_mut();
  let f = match &**next_arg!(args "buf.filter需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.map第一个参数只能传函数")
  };

  Litr::Buf(v.iter().filter_map(|&a|match scope.call(
    vec![CalcRef::Own(Litr::Uint(a as usize))], f
  ) {
    Litr::Bool(b)=> b.then(||a),
    _=> None
  }).collect::<Vec<u8>>())
}

/// 在数组范围内进行就地复制
fn copy_within(v:&mut Vec<u8>, mut args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter_mut();
  let start = to_usize(&**next_arg!(args "buf.copy_within需要传入第一个参数作为起始索引"));
  let end = to_usize(&**next_arg!(args "buf.copy_within需要传入第二个参数作为结束索引"));
  let dest = to_usize(&**next_arg!(args "buf.copy_within需要传入第三个参数作为复制目标位置索引"));
  let len = v.len();

  assert!(start <= end, "起始索引{start} 不可大于结束索引{end}");
  assert!(end <= len, "结束索引{end} 不可大于数组长度{len}");
  assert!(dest < len, "目标索引{dest} 不可大于等于数组长度{len}");

  let count = end - start;
  assert!(count <= len - dest, "可写入空间{} 不足选中长度{count}", len - dest);

  // SAFETY: 看slice::copy_within
  unsafe {
    let ptr = v.as_mut_ptr();
    let src_ptr = ptr.add(start);
    let dest_ptr = ptr.add(dest);
    std::ptr::copy(src_ptr, dest_ptr, count);
  }
  Litr::Uninit
}

/// 在指定索引处写入另一个Buf
fn write() {}

// rotate

// concat join join_str join_hex to_str

// fn splice(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
//   let mut args = args.into_iter();
//   let arg0 = next_arg!(args "splice方法至少提供一个参数");
//   Litr::Uninit
// }
