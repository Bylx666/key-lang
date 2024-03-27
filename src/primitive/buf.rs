//! buf类型的静态方法和方法
//! 
//! 同时包含了一些mem的函数

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
    b"expand"=> expand(v, args),
    b"rotate"=> rotate(v, args),
    b"concat"=> concat(v, args),
    b"concat_clone"=> concat_clone(v, args),
    b"join"=> join(v, args),
    b"fold"=> fold(v, args, scope),
    b"slice"=> slice(v, args),
    b"slice_clone"=> slice_clone(v, args),
    b"includes"=> includes(v, args),
    b"index_of"=> index_of(v, args, scope),
    b"r_index_of"=> r_index_of(v, args, scope),
    b"all"=> all(v, args, scope),
    b"min"=> min(v),
    b"max"=> max(v),
    b"part"=> part(v, args, scope),
    b"read"=> read(v, args),
    b"read_float"=> read(v, args),
    b"as_utf8"=> as_utf8(v),
    b"as_utf16"=> as_utf16(v),
    b"to_list"=> Litr::List(v.iter().map(|n|Litr::Uint(*n as usize)).collect()),
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
  assert!(args.len()>=3, "buf.copy_within需要传入3个参数:起始索引,结束索引,复制目标索引");
  let start = to_usize(args.get(0).unwrap());
  let end = to_usize(args.get(1).unwrap());
  let dest = to_usize(args.get(2).unwrap());
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

  Litr::Buf(match &**args.get(2).unwrap() {
    Litr::Buf(b)=> 
      v.splice(start..end, b.iter().copied()).collect(),
    Litr::List(b)=> 
      v.splice(start..end, b.iter().map(|n|to_u8(n))).collect(),
    n=> {
      let n = to_u8(n);
      v.splice(start..end, [n]).collect()
    }
  })
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

/// 扩大vec容量 如果空间足够可能会不做任何事
fn expand(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let n = to_usize(args.get(0).expect("buf.expand需要一个整数作为扩大字节数"));
  v.reserve(n);
  Litr::Uninit
}

/// 横向旋转数组, 相当于整体移动并将溢出值移到另一边
fn rotate(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut n = to_usize(args.get(0).expect("buf.rotate需要一个整数代表移动字节数"));
  // 使旋转大小永小于数组长度
  n %= v.len();
  // 如果第二个参数传了true就左移
  if let Some(arg1) = args.get(1) {
    if let Litr::Bool(arg1) = &**arg1 {
      if *arg1 {
        v.rotate_left(n);
        return Litr::Uninit;
      }
    }
  }
  // 否则右移
  v.rotate_right(n);
  Litr::Uninit
}

/// 将另一个Buf连接到自己后面
fn concat(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let other_tmp;
  let other = match &**args.get(0).expect("buf.concat需要传入另一个Buf或数组") {
    Litr::List(b)=> {
      other_tmp = b.iter().map(|n|to_u8(n)).collect();
      &other_tmp
    }
    Litr::Buf(b)=> b,
    n=> {
      v.push(to_u8(n));
      return Litr::Uninit;
    }
  };
  v.extend_from_slice(other);
  Litr::Uninit
}

/// concat复制版本
fn concat_clone(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let mut v = v.clone();
  let other_tmp;
  let other = match &**args.get(0).expect("buf.concat需要传入另一个Buf或数组") {
    Litr::List(b)=> {
      other_tmp = b.iter().map(|n|to_u8(n)).collect();
      &other_tmp
    }
    Litr::Buf(b)=> b,
    n=> {
      v.push(to_u8(n));
      return Litr::Uninit;
    }
  };
  v.extend_from_slice(other);
  Litr::Buf(v)
}

/// 将十六进制数以字符的格式渲染, 传入一个分隔符
fn join(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  if v.len()==0 {return Litr::Str(String::new());}

  let sep = if let Some(s) = args.get(0) {
    if let Litr::Str(s) = &**s {s}else {
      panic!("buf.join第一个参数只能是字符")
    }
  }else {""};

  use std::fmt::Write;
  let mut s = String::new();
  s.write_fmt(format_args!("{:02X}",v[0]));
  for n in &v[1..] {
    s.write_fmt(format_args!("{sep}{n:02X}"));
  }
  Litr::Str(s)
}

/// 嘎嘎复制和计算, 将整个数组折叠成一个值
fn fold(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let mut init = args.get(0).expect("buf.fold需要一个初始值").clone().own();
  let f = match &**args.get(1).expect("buf.fold需要第二个参数的函数来处理数据") {
    Litr::Func(f)=> f,
    _=> panic!("buf.fold第二个参数只能是函数")
  };
  v.iter().fold(init, |a, b|{
    scope.call(vec![CalcRef::Own(a), CalcRef::Own(Litr::Uint(*b as usize))], f)
  })
}

/// 将自己切成指定范围的数据
fn slice(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let len = v.len();
  let start = args.get(0).map_or(0, |n|to_usize(n));
  let end = args.get(0).map_or(len, |n|to_usize(n));

  assert!(start<=end, "切片起始索引{start}不可大于结束索引{end}");
  assert!(end<=len, "切片结束索引{end}不可大于数组长度{len}");

  v.copy_within(start..end, 0);
  // SAFETY: end必定小于数组长度, 因此end - start必定小于数组长度
  unsafe { v.set_len(end - start) }
  Litr::Uninit
}

/// slice的复制版本
fn slice_clone(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let len = v.len();
  let start = args.get(0).map_or(0, |n|to_usize(n));
  let end = args.get(0).map_or(len, |n|to_usize(n));

  assert!(start<=end, "切片起始索引{start}不可大于结束索引{end}");
  assert!(end<=len, "切片结束索引{end}不可大于数组长度{len}");

  Litr::Buf(v[start..end].to_vec())
}

/// 是否存在一个数
fn includes(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let find = to_u8(args.get(0).expect("buf.includes需要知道你要找啥"));
  Litr::Bool(match v.iter().find(|&&n|n==find) {
    Some(_)=> true,
    None=> false
  })
}

/// 找数组中第一个所指数字, 也可以传函数来自定义判断
fn index_of(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let res = match &**args.get(0).expect("buf.index_of需要传入一个数字或判断函数") {
    Litr::Func(f)=> {
      v.iter().position(|n|
        match scope.call(vec![CalcRef::Own(Litr::Uint(*n as usize))], f) {
          Litr::Bool(n)=> n,
          _=> false
        })
    }
    n=> {
      let find = to_u8(n);
      v.iter().position(|n|*n==find)
    }
  };
  Litr::Int(match res {
    Some(n)=> n as isize,
    None=> -1
  })
}

/// index_of的反向版本
fn r_index_of(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let res = match &**args.get(0).expect("buf.r_index_of需要知道你要找啥") {
    Litr::Func(f)=> {
      v.iter().rev().position(|n|
        match scope.call(vec![CalcRef::Own(Litr::Uint(*n as usize))], f) {
          Litr::Bool(n)=> n,
          _=> false
        })
    }
    n=> {
      let find = to_u8(n);
      v.iter().rev().position(|n|*n==find)
    }
  };
  Litr::Int(match res {
    Some(n)=> (v.len() - n - 1) as isize,
    None=> -1
  })
}

/// 测试所有元素是否都能让传入函数返回true
fn all(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.all需要传入一个函数来判断元素是否所需") {
    Litr::Func(f)=> f,
    _=> panic!("buf.all第一个参数必须是函数")
  };
  let b = v.iter().all(|n|
    match scope.call(vec![CalcRef::Own(Litr::Uint(*n as usize))], f) {
      Litr::Bool(b)=> b,
      _=> false
    });
  Litr::Bool(b)
}

/// 找最小值
fn min(v:&mut Vec<u8>)-> Litr {
  Litr::Uint(v.iter().min().copied().unwrap_or(0) as _)
}

/// 找最大值
fn max(v:&mut Vec<u8>)-> Litr {
  Litr::Uint(v.iter().max().copied().unwrap_or(0) as _)
}

/// 像是filter,但自己会变成filter剩下的内容
fn part(v:&mut Vec<u8>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.part需要传入一个函数来判断元素是否所需") {
    Litr::Func(f)=> f,
    _=> panic!("buf.part第一个参数必须是函数")
  };
  let (ret, this) = v.iter().partition(|n|
    match scope.call(vec![CalcRef::Own(Litr::Uint(**n as usize))], f) {
      Litr::Bool(b)=> b,
      _=> false
    });
  *v = this;
  Litr::Buf(ret)
}

/// 在指定偏移读取一个Uint, 第三个参数取决于机器的大小端, true不一定指大端序
fn read(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr {
  let index = args.get(0).map_or(0, |n|to_usize(n));
  let sz = args.get(1).map_or(8, |n|to_usize(n));
  let big_endian = args.get(2).map_or(false, |n|
    match &**n {
      Litr::Bool(b)=> *b,
      _=> false
    }
  );
  
  // 访问溢出时直接返回0
  if sz / 8 + index >= v.len() {
    return Litr::Uint(0);
  }

  unsafe {
    let start = v.as_ptr().add(index);
    macro_rules! imp {($($n:literal:$t:ty)*)=> {
      match sz {
        8=> *start as usize,
        $(
          $n=> {
            let n = (start as *const $t).read_unaligned();
            (if big_endian {
              n.swap_bytes()
            }else {n}) as usize
          }
        )*
        _=> panic!("buf.read第二个参数只允许8,16,32,64")
      }
    }}
    Litr::Uint(imp!(16:u16 32:u32 64:u64))
  }
}

/// 在指定偏移读取一个Float, 第二个参数取决于机器的大小端, true不一定指大端序
fn read_float(v:&mut Vec<u8>, args:Vec<CalcRef>)-> Litr { 
  let index = args.get(0).map_or(0, |n|to_usize(n));
  if index + 8 >= v.len() {
    return Litr::Float(0.0);
  }
  let big_endian = args.get(2).map_or(false, |n|
    match &**n {
      Litr::Bool(b)=> *b,
      _=> false
    }
  );

  unsafe {
    let mut f = (v.as_ptr().add(index) as *mut [u8;8]).read_unaligned();
    if big_endian {f.reverse()}
    Litr::Float(f64::from_ne_bytes(f))
  }
}

/// 以utf8解码
fn as_utf8(v:&mut Vec<u8>)-> Litr {
  Litr::Str(String::from_utf8_lossy(v).into_owned())
}

/// 以utf16解码
fn as_utf16(v:&mut Vec<u8>)-> Litr {
  let s = unsafe {
    let ptr = v.as_ptr() as *const u16;
    // 无符号数除2会自动向下取整
    let len = v.len() / 2;
    std::slice::from_raw_parts(ptr, len)
  };
  Litr::Str(String::from_utf16_lossy(s))
}


// - statics -
pub fn statics()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"new"), s_new),
    (intern(b"new_uninit"), s_new_uninit),
    (intern(b"from_list"), s_from_list),
    (intern(b"from_iter"), s_from_iter),
    (intern(b"from_ptr"), s_from_ptr),
    (intern(b"concat"), s_concat)
  ]
}

/// 创建n长度的Buf
fn s_new(args:Vec<CalcRef>, cx:Scope)-> Litr {
  // 如果传入了大小就按大小分配
  if let Some(n) = args.get(0) {
    let n = to_usize(n);

    unsafe {
      let layout = std::alloc::Layout::from_size_align_unchecked(n, 1);
      let alc = unsafe {std::alloc::alloc_zeroed(layout)};
      Litr::Buf(Vec::from_raw_parts(alc, n, n))
    }
  }else {
    Litr::Buf(Vec::new())
  }
}

/// 创建n长度未初始化数组
fn s_new_uninit(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  // 如果传入了大小就按大小分配
  if let Some(n) = args.get(0) {
    let n = to_usize(n);

    unsafe {
      let layout = std::alloc::Layout::from_size_align_unchecked(n, 1);
      let alc = unsafe {std::alloc::alloc(layout)};
      Litr::Buf(Vec::from_raw_parts(alc, n, n))
    }
  }else {
    Litr::Buf(Vec::new())
  }
}

/// 通过列表创建Buf
fn s_from_list(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let ls = match &**args.get(0).expect("Buf::from_list需要传入一个列表") {
    Litr::List(ls)=> ls,
    _=> panic!("Buf::from_list第一个参数必须是列表")
  };
  Litr::Buf(ls.iter().map(|n|to_u8(n)).collect())
}

/// 通过迭代器创建Buf
fn s_from_iter(mut args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let from = args.get_mut(0).expect("Buf::from_iter需要一个允许迭代的元素");
  let itr = iter::LitrIterator::new(&mut **from);
  Litr::Buf(itr.map(|n|to_u8(&n)).collect())
}

/// 通过指针和长度创建一个复制版的Buf
fn s_from_ptr(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  assert!(args.len()>=2, "Buf::from_ptr需要传入一个指针和一个长度");
  let from = match &*args[0] {
    Litr::Uint(n)=> {
      let n = *n;
      assert!(n!=0, "Buf::from_ptr禁止传入空指针");
      n
    }
    _=> panic!("Buf::from_ptr的指针只允许Uint类型")
  };
  let len = to_usize(&*args[1]);
  unsafe {
    Litr::Buf(std::slice::from_raw_parts(from as *const u8, len).to_vec())
  }
}

/// Buf::concat拼接两个Buf,允许传List自动转换
fn s_concat(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  assert!(args.len()>=2, "Buf::concat需要左右两个buf作参数");
  let mut left = match &**args.get(0).unwrap() {
    Litr::List(v)=> v.iter().map(|n|to_u8(n)).collect(),
    Litr::Buf(v)=> v.clone(),
    n=> vec![to_u8(n)]
  };
  match &**args.get(1).unwrap() {
    Litr::List(v)=> left.extend(v.iter().map(|n|to_u8(n))),
    Litr::Buf(v)=> left.extend_from_slice(v),
    n=> left.push(to_u8(n))
  };
  Litr::Buf(left)
}