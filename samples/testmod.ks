
let ll = 11;

class Priv {
  b:Int
}
mod:A{
  >a:Priv,>b,>c,
  .o():self,
  >ok():A::{
    a:Priv::{b:ll}
  },
}
mod.a() log(ll)
