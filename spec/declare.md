
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
let a:Type = Uninit;

我用了Uninit这个词，理解为null就好。
上述两者等价。

Uninit可以适配任何类型，并不是因为任何类型都继承它，而是因为它是大写字母开头的关键词。

在使用Uninit时，程序会提前留出其类型长度的空间，因此使用其指针是有效的。


## 常量
每次使用都会复制一份出去
const a = Type::new();
