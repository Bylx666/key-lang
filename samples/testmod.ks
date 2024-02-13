{
  let tt = "hhhh"
  mod:MyStruct {
    a,>b
    >.d(){log("ok")}
    >new():MyStruct {
      a:99
    }
  }
  let s = MyStruct::new()
}
