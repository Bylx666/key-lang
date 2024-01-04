let b = 0;
let a() {
  b = 3;
}
let a(): Int {
  b = 2;
  return b;
}
let a = || {
  b = 4;
}

{
  a(20);
}
let a(i: Int) {
  i = 5;
  let i = Int::new(i);
  let i = i.copy();
}

extern ok() {
  
}
extern(/a/a.dll) ok() {

}


自定义参数：把Expr的raw值转换一下扔进去

