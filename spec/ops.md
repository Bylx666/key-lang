Struct:method()
instance.prop
]call
`x[i]`

]unary - ! ? 取负 按位取反或逻辑取反 将未定义的变量作为uninit

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
, 列表构造符

uninit的逻辑(&&||)和false行为相同
比较类型不同时会直接得到false
比较数字时会将整数统一为Int
浮点数和整数会统一为浮点数
列表只能用==，其他比较运算符都返回false

字符+