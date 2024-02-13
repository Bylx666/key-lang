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

class Test {
  > a:Func
  > new():Test {
    c: 20
  },
  > .pubmet();
  b
  >c 
  d
  .met() {}
}
let p = Test::new()

log(p)
