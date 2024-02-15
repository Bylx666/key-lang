
class A {
  a b
  new():A{
    a:A{b:9}
  },
  .f() {
    self.a = 99;
    :self
  }
  .g() {
    log(self.a)
  }
}

let a = A::new();
{
  let f() log("ok")
  a.a.b = f
}
a.a.b()
