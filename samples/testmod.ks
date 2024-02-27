
{
  let tt = "hhhh"
  mod:MyStruct {
    a,b
    f();
    .@clone():MyStruct::{
      a: self.a+1, b: self.b+1
    },
    .@drop() {
      log("drop")
    }
    >n() {MyStruct::f()},
    >.d(){log("ok");:8},
    >new(): MyStruct::{
      a:99,b:20
    }
  }
}
