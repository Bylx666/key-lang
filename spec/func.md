参数逗号能省略...

let b = 0;
let a() b = 3;
let a():b;
let a(); // 空函数，可以当类型占位符

let a() {
  b = 2;
  return b;
}
let a = || {
  b = 4;
}
let a(a = 20) {
  log(a)
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
// 直接调用
extern kernel32> 
  GetStdHandle(n:Int)

// 别名调用
extern kernel32> {
  std: GetStdHandle(n:Int)
  write: WriteConsoleA(
    output:HANDLE,
    buffer:LPCVOID,
    charNum:DWORD,
    written:LPDWORD,
    rev
  )
}
write(std(-11),"ok",2)

// 创建一个线程
// 警告，以下例子仅供示范，多线程读写同一变量是未定义行为！
// 而且一个程序最多只能靠这种方式导出一个本地函数
extern kernel32> {
  CreateThread(a,b,c,d,e,f)
  WaitForSingleObject(a,b)
}
let f(a) {
  print(a)
}
WaitForSingleObject(CreateThread(,,f,22,,),99999)

// 自定义dll
extern C:\a\b.dll> {
  fa: FunctionA()
  fw: FunctionW();
}
```
分号可省略
`>`要紧贴文件名
需要注意，不管extern写在哪，在顶级作用域都能访问到。一是为了防止反复查询同一函数，二是如果出现在没大括号的函数体中意义不明。
DLL寻找行为和LoadLibraryA一致。

基本数据类型作为参数将自动转换。由于参数存在对齐，64位以下数值类型对参数无意义。
Bool->  (u64)1/0
Int->   (i64) signed int
Uint->  (u64) unsigned int
Float-> (f64) double float
Func->  (FARPROC) function ptr
Str->   (LPSTR) string ptr 你要自己看情况加\0
Buffer->(u64) buffer ptr 将str编码为utf16时或许会用到吧
cstruct-> (u64) ptr of cstruct
struct-> 未定义行为
Array-> 未定义行为

目前extern函数参数上限为15。extern的函数参数类型可以随便写也可以省略，编译器并不检查声明和传入类型。参数类型只与实际传入类型有关，所以参数声明还是要保证自己看得懂。
值得一提的是，由于ks只计划支持64位，所以32位的调用约定在这里并不需要指定，且内存布局中一个参数内存长度固定为8字节。
调用时省略参数将会自动传0。

## extern Func

当你传递Func作为extern函数的参数时，最多只能有7个参数。如果你的函数被调用成功，所有的参数都会是Uint类型。
换句话说，如果此函数只是给extern用的话，你的参数类型就可以随便写了，编译器不会报错的。

不过由于脚本语言天生的局限性，一个程序目前只能有一个作为extern函数参数的ks函数，后者将覆盖前者。即使可以多加，使用成本也远大于使用收益。
因此需要向外导出函数的程序请使用“底层模块”来实现。

## 可变参数

```
let my_func([args]) {
  let arg0 = args[0];
}
```

## return

return和其他编程语言行为基本一致，无显式返回就会隐式返回uninit。但需要注意的是，你可以在顶级作用域使用return，且Int返回值可以作为程序的ExitCode。

keyscript定义了一个语法糖
```
: uninit;
:;

let sth(): 20;
let sth()
  return 20;
```
可以使用冒号开头的语句代替return

## 未绑定函数

函数声明只有在运行时才会绑定作用域，未访问到的函数声明被称作"未绑定函数"。未绑定函数只有在第一次被运行到才会动态的绑定到当前作用域，而不会直接的绑定到定义处。

```
let out;
let tmp = 666
{
  let tmp = 555
  let a() {
    log("a",tmp)
    let b() log("b",tmp);
    :b
  }
  out = a
}
let binded = out(); // b函数的声明在这里才被绑定
binded(); // a 555 b 666
```

以上行为中，b函数
