KeyScript

中文名：木语

## 基础类型

表示类型大小写不限。
```
let i = 0i;                 // Int   (可省略i，代表指针长度有符号整数)
let u = 0u;                 // Uint  (不可省略u，指针长度无符号整数)
let f = 0.0l;               // Float (可省略l，通过"."标识浮点)
let f = 0l;                 // Float (不加"."时不可省略l)
let u8 = 0h;                // Byte  (不可省略h，uint8)
let u8 = 'a';               // Byte  (只允许ascii。单LF换行符也能解析)
let b = 0t;                 // Bool  (1t:true,0t:false 可以加减)
let str = "test";           // Str   (字符串字面量)
let vec = [2, 5, 6, 8];     // Array (Int类型,禁止多类型)
let buf = [29h, 28, 40, 26];// Array (u8类型)
let vec = 2, 5, 6, 8;       // Array (Int类型)

```

## 数字字面量扩展语法

```
let a = 0b1001_x; // 二进制 9 Uint8
let a = 0x2b_u;   // 十六进制 43 Uint
let a = 0xB3_f;   // 十六进制 179.0 Float
```
扩展语法中不支持小数点。必须使用下划线接类型，否则默认Int。

## 字符串扩展语法

```
let a = "\n\r\"\0";
```

## 比较

```
a == 5; // 
if a = 5 {} // a全等于5
if a >= 5 {} // a大于等于5
```

## 使用块语句初始化值

变量污染还挺烦的，所以我准备了这个

let a = {
  let a = 2;
  let b = 1;
  a + b // 不要在最后一句写分号
};

## 定义类型

let a = Type:new();
let a = Type:new();

## 未初始化

let a;
let a = uninit;

上述两行代码等价。

我用了uninit关键词，理解为null就好。uninit不占空间，使用其指针会返回0。


## 泛型

特殊类型Any可以作为函数的参数类型，代表**不检查**其类型。

Any不是关键词，但你不能在let表达式中使用Any。同时定义Any类型时不会报错，但不会生效。

你可以像js的参数用法一样，在使用Any参数时使用其实际属性或方法，并在未找到对应函数或属性时抛出异常。

你可以使用key关键词为Any指定别名，用来告诉编程者Any所期望的类型。我建议使用@前缀作为Any泛型的别名。

## 常量
每次使用都会复制一份出去
const a = Type:new();

## match
```
match a {
  a {}
  b {}
  ? {}
}
```

## 粗暴的分号省略

如果你是初学者，赋值和调用函数的语句请你不要省略分号，这是非常坏毛病。

我的编译器实现中，`let a = 0 let b = a print(b)`这样的单行写法是可以过编译的，而且会被解析成1行内的3个语句。但为了代码的美观，减少歧义，请你使用分号加换行来为你的代码明确的表现出一个语句的存在。