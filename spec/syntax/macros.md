开头点个#号，也可以直接在开头声明宏来提示编译器。

## 模板宏

就是个函数，但定义时不被上下文约束，只有在调用时才会使用调用处的上下文。

调用时的小括号可省略。

```
macro a() {
  some = 20
}
{
  let some = 5;
  #a();
  #a;
  log(some) // 20
}
```

## a