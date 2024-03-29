
]unary - ! 取负 按位取反或逻辑取反 将未定义的变量作为uninit

-. -:
-> ::
Struct:method()
instance.prop
]call
`x[i]`

* / %
+ -
>> <<
&
^
|
== != >= <= > < 别忘了做数组比较
&&
||
Assignments = += -= *= /= %= &= |= ^= <<= >>=

uninit的逻辑(&&||)和false行为相同
buffer比较依赖以下算法，以==为例的源码如下。其他比较运算符可直接原地替换==得到正确的结果。
```rust
fn compare(left: &[u8], right: &[u8]) -> bool {
  // 首先将短的buffer长度作为比较对象
  let l = min(left.len(), right.len());
  // 逐位比较，只要出现一位比较另一位为false就代表比较失败返回false
  for i in 0..l {
    if !(left[i] == right[i]) {
      return false
    }
  }
  // 若每一位都相同就比较长度，长度也相同则代表两个buffer全等
  return left.len() == right.len();
}
```

Str, Instance, List, Obj也依照以上算法比较。

对于List和Instance,遇到无法比较的类型时会提前返回false

Obj底层是哈希表,由于其无序性,使用大于和小于时一律得到false.

对于Instance, 如果两个实例不属于同一个class的话会直接返回false

赋值就是复制，不太好笑。。

使用连等：
为了减少无意义的数据复制，目前赋值语句只返回uninit，所以连等并不可用。

一元运算符优先级比二元运算符高

index[]
以下类型可以使用索引
Buffer 返回Uint,在下标无效时返回uninit
List 返回对应槽位的值
Str 获取第i个unicode字符(性能较差,请用迭代器替代)
Uint 传入小于64的数字,返回Bool,代表从右到左的二进制第i位是否为1

index不会报错,在遇到问题时会直接返回uninit

## 管道操作符

x|> |%|等同于x

x|>();
{
  let x = 5;
  log(|%|)
}
只要|>后不使用|%|, 就可以储存一段表达式, 并释放在另一处
只是储存表达式, 储存时的上下文对表达式内容无影响.

```
let a(n):n+2;
let b(n):n+3;
let c(n):n+4;
```

以下代码

```
log(a(b(c(1))));
```

可以写为

```
1|>c(|%|)
 |>b(|%|)
 |>a(|%|)
 |>log(|%|)
```