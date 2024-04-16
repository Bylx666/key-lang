
//mod D:\code\rs\tst\target\debug\tstlib.dll> m;


let a = 5;
let f1() {
  let a = 20;
  :||:a // 相当于return (||return a)
}
log(f1()()) // 20
