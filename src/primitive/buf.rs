
use super::*;

pub fn method(v:&mut Vec<u8>, scope:Scope, name:Interned, args:Vec<CalcRef>)-> Litr {
  match name.vec() {
    b"push"=> push(v, args),
    b"push_front"=> push_front(v, args),
    b"dedup"=> dedup(v, args, scope),
    b"sort"=> sort(v, args, scope),
    b"for_each"=> for_each(v, args, scope),
    b"map_clone"=> map_clone(v, args, scope),
    b"map"=> map(v, args, scope),
    b"pop"=> pop(v, args),
    b"pop_front"=> pop_front(v, args),
    b"rev"=> rev(v),
    b"filter"=> filter(v, args, scope),
    b"filter_clone"=> filter_clone(v, args, scope),
    b"copy_within"=> copy_within(v, args),
    b"write"=> write(v, args),
    b"last"=> last(v),
    b"repeat"=> repeat(v, args),
    b"repeat_clone"=> repeat_clone(v, args),
    b"insert"=> insert(v, args),
    b"insert_clone"=> insert_clone(v, args),
    b"remove"=> remove(v, args),
    b"splice"=> splice(v, args),
    b"fill"=> fill(v, args),
    b"fill_clone"=> fill_clone(v, args),
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
fn push(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  match &**args.get(0).expect("'push'方法需要一个数字,列表或数组作为参数") {
    Litr::Buf(right)=> v.extend_from_slice(right),
    Litr::List(right)=> v.extend_from_slice(
      &right.iter().map(|litr|to_u8(litr)).collect::<Box<[u8]>>()),
    n=> v.push(to_u8(n))
  };
  Litr::Uninit
}

/// 像是js的unshift
fn push_front(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  match &**args.get(0).expect("'push_front'方法需要一个数字,列表或数组作为参数") {
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
fn for_each(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.foreach需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.foreach第一个参数只能传函数")
  };
  v.iter().for_each(|a| {scope.call(vec![
    CalcRef::Own(Litr::Uint(*a as usize))
  ], f);});
  Litr::Uninit
}

/// 映射重构新Buf
fn map_clone(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.map需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.map第一个参数只能传函数")
  };
  Litr::Buf(v.iter().map(|a| match scope.call(vec![
    CalcRef::Own(Litr::Uint(*a as usize))
  ], f) {
    Litr::Uint(n)=> n as u8,
    Litr::Int(n)=> n as u8,
    _=> 0
  }).collect())
}

/// map_clone的原地版本
fn map(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.map需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.map第一个参数只能传函数")
  };
  *v = v.iter().map(|a| match scope.call(vec![
    CalcRef::Own(Litr::Uint(*a as usize))
  ], f) {
    Litr::Uint(n)=> n as u8,
    Litr::Int(n)=> n as u8,
    _=> 0
  }).collect();
  Litr::Uninit
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
fn filter(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.filter需要一个函数作为参数") {
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
fn filter_clone(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.filter需要一个函数作为参数") {
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
fn copy_within(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter();
  let start = to_usize(&**next_arg!(args "buf.copy_within需要传入第一个参数作为起始索引"));
  let end = to_usize(&**next_arg!(args "buf.copy_within需要传入第二个参数作为结束索引"));
  let dest = to_usize(&**next_arg!(args "buf.copy_within需要传入第三个参数作为复制目标位置索引"));
  let len = v.len();

  assert!(start <= end, "起始索引{start} 不可大于结束索引{end}");
  assert!(end <= len, "结束索引{end} 不可大于数组长度{len}");
  assert!(dest < len, "目标索引{dest} 不可大于等于数组长度{len}");

  let mut count = end - start;
  // 将会溢出的长度切掉
  if count > len - dest {
    count = len - dest;
  }

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
fn write(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter();

  let ls_tmp;
  let buf = match &**args.next().expect("buf.write需要一个列表或数组作为写入内容") {
    Litr::List(ls)=> {
      ls_tmp = ls.iter().map(|n|to_u8(n)).collect();
      &ls_tmp
    },
    Litr::Buf(b)=> b,
    _=> panic!("buf.write第一个参数必须是列表或数组")
  };
  let ori_len = v.len();
  let to_write_len = buf.len();

  let index = args.next().map(|v|to_usize(&**v)).unwrap_or(0);
  assert!(index<ori_len, "传入索引{index}不可大于等于数组长度{ori_len}");

  let max_count = ori_len - index;
  // 如果要写入的数组长度溢出 就改为只写入最大长度
  let count = if to_write_len < max_count {
    to_write_len
  }else {
    max_count
  };

  // SAFETE: 写入索引和写入大小都不会溢出原数组
  unsafe {
    let ori_p = v.as_mut_ptr();
    let src = buf.as_ptr();
    assert!(ori_p != src as _, "同一个数组内的复制行为请使用copy_within代替");
    let dst = ori_p.add(index);
    std::ptr::copy_nonoverlapping(src, dst, count);
  }
  Litr::Uninit
}

/// 获取最后一个数字
fn last(v:&mut Vec<u8>)-> Litr {
  v.last().map_or(Litr::Uninit, |v|Litr::Uint(*v as usize))
}

/// 将已有数组重复n次
fn repeat(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let n = to_usize(args.get(0).expect("buf.repeat需要传入整数作为重复次数"));
  *v = v.repeat(n);
  Litr::Uninit
}

/// repeat的复制版
fn repeat_clone(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let n = to_usize(args.get(0).expect("buf.repeat需要传入整数作为重复次数"));
  Litr::Buf(v.repeat(n))
}

/// 插入单数字或数组
fn insert(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter();
  let index = to_usize(&**args.next().expect("buf.insert需要传入一个数字作为插入位置"));
  assert!(index<v.len(), "插入索引{index}不可大于等于数组长度{}",v.len());

  match &**args.next().expect("buf.insert需要传入第二个参数:整数,列表或数组作为插入内容") {
    Litr::Buf(b)=> {
      v.splice(index..index, b.iter().copied()).collect::<Vec<_>>();
    },
    Litr::List(b)=> {
      v.splice(index..index, b.iter()
        .map(|n|to_u8(n))).collect::<Vec<_>>();
    }
    n=> v.insert(index, to_u8(n))
  }
  Litr::Uninit
}

/// insert的复制版本
fn insert_clone(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut v = v.clone();
  let mut args = args.iter();
  let index = to_usize(&**args.next().expect("buf.insert需要传入一个数字作为插入位置"));
  assert!(index<v.len(), "插入索引{index}不可大于等于数组长度{}",v.len());

  match &**args.next().expect("buf.insert需要传入第二个参数:整数,列表或数组作为插入内容") {
    Litr::Buf(b)=> {
      v.splice(index..index, b.iter().copied()).collect::<Vec<_>>();
    },
    Litr::List(b)=> {
      v.splice(index..index, b.iter()
        .map(|n|to_u8(n))).collect::<Vec<_>>();
    }
    n=> v.insert(index, to_u8(n))
  }
  Litr::Buf(v)
}

/// 删除一个或一段元素
fn remove(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let index = to_usize(&**args.get(0).expect("buf.remove需要一个整数作为删除索引"));
  assert!(index < v.len(), "删除索引{index}不可大于等于数组长度{}",v.len());

  // 移除多元素
  if let Some(n) = args.get(1) {
    // 防止删除索引溢出
    let mut rm_end = index + to_usize(&**n);
    if rm_end > v.len() {rm_end = v.len()}

    let removed = v.splice(index..rm_end, []).collect();
    return Litr::Buf(removed);
  }

  // 移除单元素
  Litr::Uint(v.remove(index) as usize)
}

/// remove+insert
fn splice(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  assert!(args.len()>=3, "buf.splice需要3个参数:删除起始索引,删除结束索引,要插入的内容(数组或整数)");
  let start = to_usize(args.get(0).unwrap());
  let end = to_usize(args.get(1).unwrap());
  assert!(start<=end, "起始索引{start}不可大于结束索引{end}");
  assert!(end<=v.len(), "结束索引{end}不可大于数组长度{}",v.len());

  match &**args.get(2).unwrap() {
    Litr::Buf(b)=> Litr::Buf(
      v.splice(start..end, b.iter().copied()).collect()),
    Litr::List(b)=> Litr::Buf(
      v.splice(start..end, b.iter().map(|n|to_u8(n))).collect()),
    n=> {
      let n = to_u8(n);
      v.splice(start..end, [n]);
      Litr::Uint(n as usize)
    }
  }
}

/// 将一片区域填充为指定值
fn fill(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.iter();
  let fill = args.next().map_or(0, |n|to_u8(n));
  let start = args.next().map_or(0, |n|to_usize(n));
  let end = args.next().map_or(v.len(), |n|to_usize(n));

  assert!(start<=end, "开始索引{start}不可大于结束索引{end}");
  assert!(end<=v.len(), "结束索引{end}不可大于数组长度{}",v.len());

  v[start..end].fill(fill);
  Litr::Uninit
}

/// fill的复制版本
fn fill_clone(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut v = v.clone();
  let mut args = args.iter();
  let fill = args.next().map_or(0, |n|to_u8(n));
  let start = args.next().map_or(0, |n|to_usize(n));
  let end = args.next().map_or(v.len(), |n|to_usize(n));

  assert!(start<=end, "开始索引{start}不可大于结束索引{end}");
  assert!(end<=v.len(), "结束索引{end}不可大于数组长度{}",v.len());

  v[start..end].fill(fill);
  Litr::Buf(v)
}


// fill expand(reserve)
// rotate

// concat join join_str join_hex to_str

