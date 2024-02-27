
{
  let tt = "hhhh"
  mod:MyStruct {
    a,b
    f();
    .@index_set(i, v) {
      log("index set:", i, v)
    }
    .@index_get(i) {
      log("index get", i)
    }
    >n() {MyStruct::f()},
    >.d(){log("ok");:8},
    >new(): MyStruct::{
      a:99
    }
  }
}
