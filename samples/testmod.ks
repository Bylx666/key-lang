
{
  let tt = "hhhh"
  mod:MyStruct {
    a,b
    f();
    >n() {MyStruct::f()},
    >.d(){log("ok");:8},
    >new(): MyStruct::{
      a:99
    }
  }
}
