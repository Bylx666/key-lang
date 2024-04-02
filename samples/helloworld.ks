
//mod samples/testmod.ks> m;

class A {
  a:Uint,
  .@index_get() {
    :self.a
  }
  .@index_set(i,v) {
    self.a = v
  }
}
let a = A::{a:2u};

a[0] <<= 2;
log(a)
a[0] >>= 3;
log(a)