## 定义结构

逗号可以省略

struct A {
  a: Type,
  b: Type,
}

struct也不强制类型，在scan期就应该把->运算符右值转换为索引

## 定义方法

struct B {
  a: Type
  b: Type
}
impl B {
  new(): B {
    B {}
  }
  method(self, b:Uint) {

  }
}

## 别名

key A = B;

## obj

希望能和js的Object玩起来手感差不多

let obj = {
  a
  b: 24
}
