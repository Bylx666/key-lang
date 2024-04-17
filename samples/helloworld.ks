
//mod D:\code\rs\tst\target\debug\tstlib.dll> m;

let s = 99;

match s {
  >=99, <90 {
    log("我介于99(包含)和100(不含)之间")
  }
}

match s {
  =99, =100 {
    log("我就是99")
  }
}

match s {
  // 第一个条件是等号的话, 
  // 该条件之后所有条件满足一个即可匹配成功
  =0, >80 {
    log("我大于80");
    // 99匹配成功
  }
}
