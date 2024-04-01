let a=0;
let b=0;
let initer;
{ // 模拟作用域变化
  let a = 99;
  let b = 32;
  let f() {
    a = 20;
    b = 10;
  }
  initer = f;
}

// 两种情况下的a,b值
initer(); // a == (), b == ();
log(a,b);
initer.call_here(()) // 第一个参数代表函数内的self, 传个uninit就行
                     // a == 20, b == 10
log(a,b)