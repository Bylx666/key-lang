/`
0{
  let a = 29
  0{
    0let c(): "i am being collecting";
    ->0
  }
  a // Corruption
}

0{
  let a = 20
  0{
    let out
    let a = 29
    0{
      let c(): "i am being collecting";
      c;
      out = c
    }
    out()
  }
  a // corruption
}`/

let out;
{
  let tmp = 555
  let a() {
    log(tmp)
  }
  a()
  {
    let in = a
  }
  out = a
}
out()

mod D:\code\rs\key-lang\samples\testmod.ks> mym
mym-.test()
