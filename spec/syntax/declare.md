KeyScript

中文名：木语

## 基础类型

```
let uni = uninit;           // Uninit(就是null,但“未定义”更适合其在脚本语言的地位)
let uni = ();               // Uninit
let i = 0i;                 // Int   (可省略i，代表指针长度有符号整数)
let u = 0u;                 // Uint  (不可省略u，指针长度无符号整数)
let f = 0.0l;               // Float (可省略l，通过"."标识浮点)
let f = 0l;                 // Float (不加"."时不可省略l)
let b = true;               // Bool  (true false)
let str = "test";           // Str   (字符串字面量, 所见即所得)
let str = `\n\rtest"`       // Str   (带转义的字符串, 字面量中有双引号就用这个)
let vec = [2, 5, 6, 8, ];   // List  (叫作列表，任意类型)
let vec = [8u, true, 26];   // List 
let buf = 'Genshin{0F20}';  // Buffer(u8 {}内允许空格换行)

```


## 注释
```
// 单行注释
/'
多行注释
多行注释
'/
```
多行注释的结尾``/`省略的话就可以自动注释到文件结尾。

## 字符串扩展语法

```
let a = `\n\r\t\0\\`;
let a = `first:1;\
         second:2;` // 在一行上紧连着
let a = `this is \{some_var};` // 捕获变量
```

## 比较

```
a == 5;
a >= 5
```

## iife

let a = ||{
  let a = 2;
  let b = 1;
  return a + b;
}();

## 定义类型

let a = Type::new();
let a = Type::new();

## 未初始化

let a;
let a = uninit;

上述两行代码等价。

## 泛型

特殊类型Any可以作为函数的参数类型，代表**不检查**其类型。

Any不是关键词，但你不能在let表达式中使用Any。同时定义Any类型时不会报错，但不会生效。

你可以像js的参数用法一样，在使用Any参数时使用其实际属性或方法，并在未找到对应函数或属性时抛出异常。

你可以使用key关键词为Any指定别名，用来告诉编程者Any所期望的类型。我建议使用@前缀作为Any泛型的别名。


## keys
key关键词将一个标识符替换到另一标识符(变量或者类型都可以)

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