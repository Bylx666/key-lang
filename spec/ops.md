
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

buffer比较依赖Rust底层实现，源码如下
```rust
fn compare(left: &[u8], right: &[u8]) -> Ordering {
  // 首先将短的buffer长度作为比较对象
  let l = core::cmp::min(left.len(), right.len());
  // 使用slice消除编译器的边界检查
  let lhs = &left[..l];
  let rhs = &right[..l];
  // 逐位比较，只要出现一位大于另一位就作为结果返回
  for i in 0..l {
    match lhs[i].cmp(&rhs[i]) {
      Ordering::Equal => (),
      non_eq => return non_eq,
    }
  }
  // 若每一位都相同就比较长度，长度也相同则代表两个buffer全等
  left.len().cmp(&right.len())
}
```

列表比较是不太可靠的，首先列表内只允许数字和bool，其次要保证两列表长度相同，还要保证每一位的元素基本类型相同。因此如果数字都确定不大于255可以选用buffer比较。如果数字普遍偏大则可以考虑使用u32 buffer之类的实现。

赋值就是复制，不太好笑。。

使用连等：
为了减少无意义的数据复制，目前赋值语句只返回uninit，所以连等并不可用。

一元运算符优先级比二元运算符高