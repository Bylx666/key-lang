参数逗号能省略...

let b = 0;
let a() b = 3;
let a(); // 空函数，可以当类型占位符

let a() {
  b = 2;
  return b;
}
let a = || {
  b = 4;
}

let a(i: Int) {
  i = 5;
  let i = Int::new(i);
  let i = i.copy();
}

## 值得注意

虽然分号可以省略，但别忘了换行符不是语法。

let a()
a()

这可是定义了调用自己的函数，而不是空函数。空函数务必使用分号结尾。

## iife

let a = !{
  return 5;
}
等价于||{}()

## dll
```
extern kernel32> 
  GetStdHandle(n:Int)
extern kernel32> 
  handler: GetStdHandle(n:Int)
extern C:\a\b.dll> {
  fa: FunctionA()
  fw: FunctionW();
}
```
分号可省略
`>`要紧贴文件名
需要注意，不管extern写在哪，在顶级作用域都能访问到。一是为了防止反复查询同一函数，二是如果出现在没大括号的函数体中意义不明。
DLL寻找行为和LoadLibraryA一致。

基本数据类型作为参数将自动转换。由于参数存在对齐，64位以下数值类型无意义，请手动按位与求值。
Bool->  (u64)1/0
Int->   (i64) signed int
Uint->  (u64) unsigned int
Float-> (f64) double float
Func->  (FARPROC) function ptr
Str->   (LPSTR) string ptr 你要自己加\0
Buffer->(u64) buffer ptr 将str编码为utf16时或许会用到吧
cstruct-> (u64) ptr of cstruct
struct-> 未定义行为
Array-> 未定义行为

目前extern函数参数上限为15。extern的函数参数类型可以随便写，编译器并不检查声明和传入类型。参数类型只与实际传入类型有关，所以参数声明还是要保证自己看得懂。
值得一提的是，由于ks只计划支持64位，所以32位的调用约定在这里并不需要指定，且内存布局中一个参数内存长度固定为8字节。

## extern Func

当你传递Func作为extern函数的参数时，最多只能有15个参数。如果你的函数被调用成功，所有的参数都会是Uint类型。
换句话说，如果此函数只是给extern用的话，你的参数类型就可以随便写了，编译器不会报错的。

## 可变参数

参数并不是真的一个一个的参数，而是一个列表(Array)，由编译器自动为每个参数名赋值后给程序员用的。你完全可以不声明参数，发挥脚本语言的特长，直接使用原参数列表。

原参数(Raw Arguments): (函数名).args()

let my_func() {
  let args = my_func.args();
  let arg0 = args[0];
}

## return

return和其他编程语言行为基本一致，无显式返回就会隐式返回uninit。但需要注意的是，你可以在顶级作用域使用return，且Int返回值可以作为程序的ExitCode。
