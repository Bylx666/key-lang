{
  let tt = "hhhh"
  mod:MyStruct {
    >a
    b
    .d(){log("ok")}
    >new():MyStruct {
      b:20
    }
  }
  let s = MyStruct::new()
}
