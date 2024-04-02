
//mod samples/testmod.ks> m;

class MyVec {
  a:Buf
  .@index_get(i): self.a[i], // 别忘了逗号
  .@index_set(i, v) {
    self.a[i] = v;
  }
}
let a = MyVec::{a:'233'};
let a = '233';
a[1] = 0;
log(a);