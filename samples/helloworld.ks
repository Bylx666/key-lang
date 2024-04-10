mod D:\code\rs\tst\target\debug\tstlib.dll> m;

let f() {
  a += 2;
  log(a);
}

{
  let a = 3;
  m-.set_timeout_here(f, 2000);
}
