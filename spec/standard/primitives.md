## bool

唯一属性rev

true.rev == !true

方法rev, then

then会转达内部函数的返回值
false.rev().then(||:20) == 20

false.rev == false.rev() == true

## Func

属性type, 值是Str: local extern native
raw: Uint,代表了未绑定作用域的函数的指针,只可用于比较绑定了不同作用域的同一函数

## List

len capacity

## Str

len char_len(低性能) lines(低性能) capacity

