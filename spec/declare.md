
## 基础类型

以下写法相当于类型声明写在数值后面。
```
let i = 0i; // Int
let u = 0u?; // Uint
let f = 0.0f?; // Float
let f = 0f; // 同上
let b = 1b; // Bool
let str = ""; // Str
let vec:[Int] = [2, 5, 6, 8]; // Int Vec 类型不能缺
```

## 使用块语句初始化值

变量污染还挺烦的，所以我准备了这个

let a = {
  let a = 2;
  let b = 1;
  a + b // 不要在最后一句写分号
};

## 定义类型

let a = Type::new();
let a:Type = Type::new();

## 未初始化

let a:Type;
let a:Type = uninit;

上述两行代码等价。

我用了uninit这个词，理解为null就好。uninit可以适配任何类型。

在使用uninit时，程序会提前留出其类型长度的空间，因此使用其指针是有效的。


## 泛型

特殊类型Any可以作为函数的参数类型，代表**不检查**其类型。

Any不是关键词，但你不能在let表达式中使用Any。同时定义Any类型时不会报错，但不会生效。

你可以像js的参数用法一样，在使用Any参数时使用其实际属性或方法，并在未找到对应函数或属性时抛出异常。

你可以使用key关键词为Any指定别名，用来告诉编程者Any所期望的类型。我建议使用@前缀作为Any泛型的别名。

## 常量
每次使用都会复制一份出去
const a = Type::new();
