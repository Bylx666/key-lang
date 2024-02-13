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

mod D:\code\rs\key-lang\samples\testmod.ks> mym
let s = mym-:MyStruct::new();
log(s)
mym-:MyStruct::d(s);

