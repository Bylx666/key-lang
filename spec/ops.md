
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
比较数字时会将整数统一为Int
浮点数和整数会统一为浮点数

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

赋值就是复制，不太好笑。。

使用连等：
为了减少无意义的数据复制，目前赋值语句只返回uninit，所以连等并不可用。

一元运算符优先级比二元运算符高