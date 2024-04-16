
//mod D:\code\rs\tst\target\debug\tstlib.dll> m;

let inner() {
  // 该函数的上下文里是没有i的
  return 20;
}

{
  // i实际在此定义
  let i = 0;
  log(inner.unzip());
  return 99
}
