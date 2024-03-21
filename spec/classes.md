## 定义结构

逗号可以省略
首字母建议大写
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
  new(): MyClass::{a:"a",b:2},
  .get_a(): self.a
}
MyClass::new().get_a() == "a"


## 模块化

mod: MyClass {
  ...
}

只有\>前缀的成员才能被模块外访问。使用class而不是mod:时，>前缀无意义。
```
mod other.ks> mymod

class A = mymod-:MyClass

my_mod-:MyClass::some()

let some = my_mod-:MyClass::some;
some();
```

## 分隔符

class成员间使用`,`分隔，一般可以省略。但以下例子会报错，因为程序会将new()内容解析为A{}.met()，然后下一个字符为`{`就会提示未闭合的大括号。解决方法就是在A{}后加逗号。
```
class A {
  new(): A{}
  .met() {}
}
```

## obj

希望能和js的Object玩起来手感差不多

let obj = {
  a
  b: 24
}

Obj::insert has remove

obj["a"] = xx 等效于insert

## @xx

@index_get index_set clone drop

## is

使用is来判断实例类型(类似typeof + instanceof)
