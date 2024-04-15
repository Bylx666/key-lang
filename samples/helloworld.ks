
//mod D:\code\rs\tst\target\debug\tstlib.dll> m;


// 定义一个有a和b两个属性的本地类型A
class A {
  a b
}

// 创建一个A的实例
let inst = A::{a:9, b:20};
// 解构A实例得到a和b
let {a b} = inst;
log(a,b); // 9 20
