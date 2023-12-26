## 定义结构

key A {
  a: Type,
  b: Type
}

## 定义方法

key B {
  a: Type,
  b: Type
}
impl B {
  new(): B {
    B {}
  }
  method(self, b:Uint) {

  }
}

## 继承方法(已废弃草案)

key A extends B {
  c(self) {
    self.method();
  }
}

key A extends B;
impl A {

}

值得一提的是，你不能在此基础上加别的属性。你只能通过定义新结构并包裹这个类来加别的属性。

## 别名

key A = B;

