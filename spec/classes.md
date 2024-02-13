## 定义结构

逗号可以省略
首字母必须大写
```
class MyClass {
  a
  b:Func
  c:Str
}
```
## 定义方法
```
class MyClass {
  // 属性
  a
  // 静态函数
  static1() {}
  // 方法
  .method1() {}
}
```
## 公开性
```
class MyClass {
  // 公开
  >a
  // 私有
  b
  // 公开
  >static1() {}
  // 私有
  static2() {}
  // 公开
  >.method1() {}
  // 私有
  .method2() {}
}
```

## 创建实例

class MyClass {
  a b
  new(): MyClass {a:"a",b:2}
  .get_a(): self.a
}
MyClass::new().get_a() == "a"

## obj

希望能和js的Object玩起来手感差不多

let obj = {
  a
  b: 24
}
