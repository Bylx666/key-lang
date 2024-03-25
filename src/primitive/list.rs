//! list类型的方法(不就是buf的阉割版么)
use super::*;

pub fn method(v:&mut Vec<Litr>, scope:Scope, name:Interned, args:Vec<CalcRef>)-> Litr {
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
    b"last"=> last(v),
    b"insert"=> insert(v, args),
    b"insert_many"=> insert_many(v, args),
    b"insert_many_clone"=> insert_many_clone(v, args),
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
    b"find"=> find(v, args, scope),
    b"r_find"=> r_find(v, args, scope),
    b"all"=> all(v, args, scope),
    b"min"=> min(v),
    b"max"=> max(v),
    b"to_buf"=> to_buf(v),
    _=> panic!("List没有{}方法",name)
  }
}

const fn to_usize(v:&Litr)-> usize {
  match v {
    Litr::Int(n)=> *n as usize,
    Litr::Uint(n)=> *n,
    _=> 0
  }
}

/// 推进单个元素
fn push(v:&mut Vec<Litr>, mut args:Vec<CalcRef>)-> Litr {
  let e = args.get_mut(0).expect("list.push需要一个要推进的元素").take();
  v.push(e);
  Litr::Uninit
}

/// 像是js的unshift
fn push_front(v:&mut Vec<Litr>, mut args:Vec<CalcRef>)-> Litr {
  let mut new_v = Vec::with_capacity(v.len() + 1);
  new_v.push(args.get_mut(0).expect("list.push_front需要一个要推进的元素").take());
  new_v.extend_from_slice(v);
  *v = new_v;
  Litr::Uninit
}

/// 去重 建议和sort联用
fn dedup(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  // 如果传了第一个参数就用dedup_by
  if let Some(f) = args.get(0) {
    let f = match &**f {
      Litr::Func(f)=> f,
      _=> panic!("list.dedup第一个参数只能传函数")
    };
    v.dedup_by(|a,b| match scope.call(vec![
      CalcRef::Ref(a), CalcRef::Ref(b)
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
fn sort(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  // 如果传了第一个参数就用sort_by
  if let Some(f) = args.get(0) {
    let f = match &**f {
      Litr::Func(f)=> f,
      _=> panic!("list.sort第一个参数只能传函数")
    };
    use std::cmp::Ordering;
    v.sort_unstable_by(|a,b| match scope.call(vec![
      CalcRef::Own(a.clone()), CalcRef::Own(b.clone())
    ], f) {
      Litr::Bool(b)=> match b {
        true=> Ordering::Greater,
        false=> Ordering::Less,
      },
      _=> Ordering::Greater
    });
  }else {
    v.sort_unstable_by(|a,b|
      a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
  }
  Litr::Uninit
}

/// 循环调用
fn for_each(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.foreach需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.foreach第一个参数只能传函数")
  };
  v.iter_mut().for_each(|a| {scope.call(vec![
    CalcRef::Ref(a)
  ], f);});
  Litr::Uninit
}

/// 映射重构新Buf
fn map_clone(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.map需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.map第一个参数只能传函数")
  };
  Litr::List(v.iter_mut()
    .map(|a| scope.call(vec![CalcRef::Ref(a)], f) ).collect())
}

/// map_clone的原地版本
fn map(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("buf.map需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("buf.map第一个参数只能传函数")
  };
  *v = v.iter_mut().map(|a| scope.call(vec![CalcRef::Ref(a)], f)).collect();
  Litr::Uninit
}

/// 从末尾切去一个, 可传一个数字作为切去数量
fn pop(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  if let Some(arg0) = args.get(0) {
    let at = match &**arg0 {
      Litr::Uint(n)=> *n,
      Litr::Int(n)=> *n as usize,
      _=> panic!("list.pop的参数必须为整数")
    };
    if at >= v.len() {
      panic!("分界线索引{at}大于数组长度{}", v.len());
    }

    Litr::List(v.split_off(v.len() - at))
  }else {
    match v.pop() {
      Some(n)=> n,
      None=> Litr::Uninit
    }
  }
}

/// 从开头切去一个, 可传一个数字作为切掉数量
fn pop_front(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  if let Some(arg0) = args.get(0) {
    let at = match &**arg0 {
      Litr::Uint(n)=> *n,
      Litr::Int(n)=> *n as usize,
      _=> panic!("list.pop_front的参数必须为整数")
    };
    if at >= v.len() {
      panic!("分界线索引{at}大于数组长度{}", v.len());
    }

    let mut part = v.split_off(at);
    std::mem::swap(v, &mut part);
    Litr::List(part)
  }else {
    if v.len()==0 {return Litr::Uninit;}
    v.remove(0)
  }
}

/// 反转Buf
fn rev(v:&mut Vec<Litr>)-> Litr {
  v.reverse();
  Litr::Uninit
}

/// 将当前数组按函数过滤
fn filter(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("list.filter需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("list.map第一个参数只能传函数")
  };
  v.retain_mut(|a|match scope.call(
    vec![CalcRef::Ref(a)], f
  ) {
    Litr::Bool(b)=> b,
    _=> false
  });
  Litr::Uninit
}

/// filter的复制版本
fn filter_clone(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("list.filter需要一个函数作为参数") {
    Litr::Func(f)=> f,
    _=> panic!("list.map第一个参数只能传函数")
  };

  Litr::List(v.iter_mut().filter_map(|a|match scope.call(
    vec![CalcRef::Ref(a)], f
  ) {
    Litr::Bool(b)=> b.then(||a.clone()),
    _=> None
  }).collect())
}

/// 获取最后一个数字
fn last(v:&mut Vec<Litr>)-> Litr {
  v.last().map_or(Litr::Uninit, |v|v.clone())
}

/// 插入单个元素
fn insert(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.into_iter();
  let index = to_usize(&*args.next().expect("list.insert需要传入一个数字作为插入位置"));
  assert!(index<v.len(), "插入索引{index}不可大于等于数组长度{}",v.len());

  let to_insert = args.next().expect("list.insert需要传入第二个参数作为插入内容").own();
  v.insert(index, to_insert);
  Litr::Uninit
}

/// 插入多个元素
fn insert_many(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let index = to_usize(&**args.get(0).expect("list.insert_many需要传入一个数字作为插入位置"));
  assert!(index<v.len(), "插入索引{index}不可大于等于数组长度{}",v.len());

  match &**args.get(0).expect("list.insert_many需要传入第二个参数作为插入内容") {
    Litr::Buf(b)=> {
      v.splice(index..index, b.iter().map(|n|Litr::Uint(*n as usize))).collect::<Vec<_>>();
    },
    Litr::List(b)=> {
      v.splice(index..index, b.iter()
        .map(|n|n.clone())).collect::<Vec<_>>();
    }
    _=> panic!("list.insert_many第二个参数必须是List或Buf")
  }
  Litr::Uninit
}

/// insert_may的复制版本
fn insert_many_clone(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut v = v.clone();
  let index = to_usize(&**args.get(0).expect("list.insert_many_clone需要传入一个数字作为插入位置"));
  assert!(index<v.len(), "插入索引{index}不可大于等于数组长度{}",v.len());

  match &**args.get(0).expect("list.insert_many_clone需要传入第二个参数作为插入内容") {
    Litr::Buf(b)=> {
      v.splice(index..index, b.iter().map(|n|Litr::Uint(*n as usize))).collect::<Vec<_>>();
    },
    Litr::List(b)=> {
      v.splice(index..index, b.iter()
        .map(|n|n.clone())).collect::<Vec<_>>();
    }
    _=> panic!("list.insert_many_clone第二个参数必须是List或Buf")
  }
  Litr::List(v)
}


/// 删除一个或一段元素
fn remove(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let index = to_usize(&**args.get(0).expect("list.remove需要一个整数作为删除索引"));
  assert!(index < v.len(), "删除索引{index}不可大于等于数组长度{}",v.len());

  // 移除多元素
  if let Some(n) = args.get(1) {
    // 防止删除索引溢出
    let mut rm_end = index + to_usize(&**n);
    if rm_end > v.len() {rm_end = v.len()}

    let removed = v.splice(index..rm_end, []).collect();
    return Litr::List(removed);
  }

  // 移除单元素
  v.remove(index)
}

/// remove+insert
fn splice(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  assert!(args.len()>=3, "list.splice需要3个参数:删除起始索引,删除结束索引,要插入的列表或数组");
  let start = to_usize(args.get(0).unwrap());
  let end = to_usize(args.get(1).unwrap());
  assert!(start<=end, "起始索引{start}不可大于结束索引{end}");
  assert!(end<=v.len(), "结束索引{end}不可大于数组长度{}",v.len());

  Litr::List(match &**args.get(2).unwrap() {
    Litr::Buf(b)=>
      v.splice(start..end, b.iter().map(|n|Litr::Uint(*n as usize))).collect(),
    Litr::List(b)=> 
      v.splice(start..end, b.iter().map(|n|n.clone())).collect(),
    n=> 
      v.splice(start..end, [n.clone()].into_iter()).collect()
  })
}

/// 将一片区域填充为指定值
fn fill(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.into_iter();
  let fill = args.next().map_or(Litr::Uninit, |n|n.own());
  let start = args.next().map_or(0, |n|to_usize(&n));
  let end = args.next().map_or(v.len(), |n|to_usize(&n));

  assert!(start<=end, "开始索引{start}不可大于结束索引{end}");
  assert!(end<=v.len(), "结束索引{end}不可大于数组长度{}",v.len());

  v[start..end].fill(fill);
  Litr::Uninit
}

/// fill的复制版本
fn fill_clone(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut v = v.clone();
  let mut args = args.into_iter();
  let fill = args.next().map_or(Litr::Uninit, |n|n.own());
  let start = args.next().map_or(0, |n|to_usize(&n));
  let end = args.next().map_or(v.len(), |n|to_usize(&n));

  assert!(start<=end, "开始索引{start}不可大于结束索引{end}");
  assert!(end<=v.len(), "结束索引{end}不可大于数组长度{}",v.len());

  v[start..end].fill(fill);
  Litr::List(v)
}

/// 扩大vec容量 如果空间足够可能会不做任何事
fn expand(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let n = to_usize(args.get(0).expect("list.expand需要一个整数作为扩大字节数"));
  v.reserve(n);
  Litr::Uninit
}

/// 横向旋转数组, 相当于整体移动并将溢出值移到另一边
fn rotate(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut n = to_usize(args.get(0).expect("list.rotate需要一个整数代表移动字节数"));
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
fn concat(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut args = args.into_iter();
  let other:Vec<Litr> = match args.next().expect("list.concat需要传入另一个Buf或数组").own() {
    Litr::List(b)=> b.into_iter().collect(),
    Litr::Buf(b)=> b.into_iter().map(|n|Litr::Uint(n as usize)).collect(),
    n=> {
      v.push(n);
      return Litr::Uninit;
    }
  };
  v.extend_from_slice(&*other);
  Litr::Uninit
}

/// concat复制版本
fn concat_clone(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let mut v = v.clone();
  let mut args = args.into_iter();
  let other:Vec<Litr> = match args.next().expect("list.concat_clone需要传入另一个Buf或数组").own() {
    Litr::List(b)=> b.into_iter().collect(),
    Litr::Buf(b)=> b.into_iter().map(|n|Litr::Uint(n as usize)).collect(),
    n=> {
      v.push(n);
      return Litr::Uninit;
    }
  };
  v.extend_from_slice(&*other);
  Litr::List(v)
}

/// 将十六进制数以字符的格式渲染, 传入一个分隔符
fn join(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  if v.len()==0 {return Litr::Str(String::new());}

  let sep = if let Some(s) = args.get(0) {
    if let Litr::Str(s) = &**s {s}else {
      panic!("list.join第一个参数只能是字符")
    }
  }else {""};

  use std::fmt::Write;
  let mut res = String::new();
  res.write_fmt(format_args!("{}", v[0].str()));
  for n in &mut v[1..] {
    if let Litr::Str(s) = n {
      res.write_fmt(format_args!("{sep}{}",s));
    }else {
      res.write_fmt(format_args!("{sep}{}",n.str()));
    }
  }
  Litr::Str(res)
}

/// 嘎嘎复制和计算, 将整个数组折叠成一个值
fn fold(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let mut args = args.into_iter();
  let mut init = args.next().expect("list.fold需要一个初始值").clone().own();
  let f_ = args.next().expect("list.fold需要第二个参数的函数来处理数据");
  let f = match &*f_ {
    Litr::Func(f)=> f,
    _=> panic!("list.fold第二个参数只能是函数")
  };
  v.iter_mut().fold(init, |a, b|{
    scope.call(vec![CalcRef::Own(a), CalcRef::Ref(b)], f)
  })
}

/// 将自己切成指定范围的数据
fn slice(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let len = v.len();
  let start = args.get(0).map_or(0, |n|to_usize(n));
  let end = args.get(0).map_or(len, |n|to_usize(n));

  assert!(start<=end, "切片起始索引{start}不可大于结束索引{end}");
  assert!(end<=len, "切片结束索引{end}不可大于数组长度{len}");

  *v = v[start..end].to_vec();
  Litr::Uninit
}

/// slice的复制版本
fn slice_clone(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let len = v.len();
  let start = args.get(0).map_or(0, |n|to_usize(n));
  let end = args.get(0).map_or(len, |n|to_usize(n));

  assert!(start<=end, "切片起始索引{start}不可大于结束索引{end}");
  assert!(end<=len, "切片结束索引{end}不可大于数组长度{len}");

  Litr::List(v[start..end].to_vec())
}

/// 是否存在一个数
fn includes(v:&mut Vec<Litr>, args:Vec<CalcRef>)-> Litr {
  let find = args.get(0).expect("list.includes需要知道你要找啥");
  Litr::Bool(match v.iter().find(|&n|n==&**find) {
    Some(_)=> true,
    None=> false
  })
}

/// 找数组中第一个所指数字, 也可以传函数来自定义判断
fn index_of(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let find = &**args.get(0).expect("list.index_of需要传入一个值");
  let res = v.iter().position(|n|n==find);
  Litr::Int(match res {
    Some(n)=> n as isize,
    None=> -1
  })
}

/// index_of反向版
fn r_index_of(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let find = &**args.get(0).expect("list.r_index_of需要传入一个值");
  let res = v.iter().rev().position(|n|n==find);
  Litr::Int(match res {
    Some(n)=> (v.len() - n - 1) as isize,
    None=> -1
  })
}

/// 通过判断函数找到第一个对应值
fn find(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let find = &**args.get(0).expect("list.find需要传入一个函数");
  let res = if let Litr::Func(f) = find {
    v.iter_mut().position(|n|
      match scope.call(vec![CalcRef::Ref(n)], f) {
        Litr::Bool(n)=> n,
        _=> false
      }
    )
  }else {panic!("list.find第一个参数必须是函数")};
  res.map_or(Litr::Uninit, |n|v[n].clone())
}

/// 通过判断函数找到第一个对应值
fn r_find(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let find = &**args.get(0).expect("list.r_find需要传入一个函数");
  let res = if let Litr::Func(f) = find {
    v.iter_mut().rev().position(|n|
      match scope.call(vec![CalcRef::Ref(n)], f) {
        Litr::Bool(n)=> n,
        _=> false
      }
    )
  }else {panic!("list.r_find第一个参数必须是函数")};
  res.map_or(Litr::Uninit, |n|v[v.len() - n - 1].clone())
}

/// 测试所有元素是否都能让传入函数返回true
fn all(v:&mut Vec<Litr>, args:Vec<CalcRef>, scope:Scope)-> Litr {
  let f = match &**args.get(0).expect("list.all需要传入一个函数来判断元素是否所需") {
    Litr::Func(f)=> f,
    _=> panic!("list.all第一个参数必须是函数")
  };
  let b = v.iter_mut().all(|n|
    match scope.call(vec![CalcRef::Ref(n)], f) {
      Litr::Bool(b)=> b,
      _=> false
    });
  Litr::Bool(b)
}

/// 找最小值
fn min(v:&mut Vec<Litr>)-> Litr {
  v.iter().cloned().min_by(|a,b|a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(Litr::Uninit)
}

/// 找最大值
fn max(v:&mut Vec<Litr>)-> Litr {
  v.iter().cloned().max_by(|a,b|a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(Litr::Uninit)
}

/// List转Buf
fn to_buf(v:&mut Vec<Litr>)-> Litr {
  Litr::Buf(v.iter().map(|n|match n {
    Litr::Int(n)=> *n as u8,
    Litr::Uint(n)=> *n as u8,
    _=> 0
  }).collect())
}

// - statics -
pub fn statics()-> Vec<(Interned, NativeFn)> {
  vec![
    (intern(b"new"), s_new),
    (intern(b"from_iter"), s_from_iter),
    (intern(b"from_buf"), s_from_buf)
  ]
}

/// 创建n长度的List, 可传入初始值
fn s_new(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let mut args = args.into_iter();
  // 如果传入了大小就按大小分配
  if let Some(n) = args.next() {
    let init = if let Some(init) = args.next() {
      init.own()
    }else {Litr::Uninit};

    let n = to_usize(&n);
    let mut v = Vec::with_capacity(n);
    for space in v.spare_capacity_mut() {
      space.write(init.clone());
    }
    unsafe {v.set_len(n)}
    Litr::List(v)
  }else {
    Litr::Buf(Vec::new())
  }
}

/// 通过iter创建List
fn s_from_iter(mut args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let from = args.get_mut(0).expect("List::from_iter需要一个可迭代的元素");
  Litr::List(iter::LitrIterator::new(&mut **from).collect())
}

/// 通过buf创建List,相当于buf.to_list
fn s_from_buf(args:Vec<CalcRef>, _cx:Scope)-> Litr {
  let from = args.get(0).expect("List::from_buf需要一个Buf");
  if let Litr::Buf(v) = &**from {
    Litr::List(v.iter().map(|n|Litr::Uint(*n as usize)).collect())
  }else {panic!("List::from_buf第一个参数必须是Buf")}
}
