{
  let tt = "hhhh"
  mod:MyStruct {
    a,>b
    >.d(){log("ok");:8}
    >new():MyStruct {
      b:MyStruct {b:20}
    }
  }
}
