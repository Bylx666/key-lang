{
  let tt = "hhhh"
  mod:MyStruct {
    a,>b
    >.d(){log("ok");:8}
    >new():MyStruct {
      a:99
      b:22
    }
  }
  let s = MyStruct::new()
}
