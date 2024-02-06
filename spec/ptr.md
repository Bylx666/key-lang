## 精简的垃圾回收

概念：本地函数：在Ks程序中使用`let`或`mod.`定义的本地函数。

Ks所有的变量都和作用域绑定，因此垃圾回收行为就是回收作用域行为。而能引用作用域内容的只有本地函数，因此问题又演变成了对本地函数的追踪。

每个作用域拥有一个引用计数，声明函数时函数会被Ks绑定在作用域上。当你定义函数，或使用`mod.`导出函数, `let`将函数赋给其他作用域或直接把函数作为参数传走等行为，函数指针明显会被复制的时候，就会沿着其定义处的作用域往上，给每个上层作用域加一层引用计数。

本地函数也属于变量，当本地函数离开当前作用域时，只要它不是当前作用域定义的，就会沿着它原来定义的作用域向上，给每个作用域引用计数减少一层，并将引用计数为0的作用域回收。

## 复制行为

赋值皆复制。
将变量传进函数作为参数时也有隐式的赋值，也就是说函数得到的参数都是复制而来的。
返回值也必定被复制，因此如果函数只是处理字符，可以使用transfer或Ref声明。

## api

程序并不保证你的指针是否正确。

let a = 0u;
let p:Uint = Mem::ptr(a); // ptr to a
let p:Uint = Mem::alloc(20); // ptr to allocator

let p = 0;
p = 0;

```
Mem::sizeof(Uint); // You cannot bring Types into any other funcs!
let a:Uint = Mem::read(p); // type is nessasery
Mem::write(p, n);
Mem::leak(p);
Mem::drop(p);
Mem::typeof(a);
Mem::alloc(20);
```

## transmutation
```
let a = 0u;
let p = Mem::ptr(a);
let f:0i = Mem::read(a);
```
why not `a.into_int()`?