
{
  let tt = "hhhh"
  mod:Sample {
    >a,b
    f();
    >.d(){log(self.b)},
    >new(): Sample::{
      a:99,b:20
    }
  }
    2/0;
}
