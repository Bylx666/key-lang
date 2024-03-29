
## 方法自动转换

当需要传入整数时, 只要传的不是整数都会自动转为0.

## 基本类型的方法命名

一般来说, 返回值和自己类型相同的方法都会区分出clone版本, 请注意带有clone版本的方法往往不带clone的版本都会直接操作自己. 返回值类型不同时往往不会修改自己的值.

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

ptr byte_len len(utf8的字符数,低性能) lines(低性能) capacity

## 数字 Int Uint Float

都有int,uint,float属性,你可以在不确定数字类型时统一使用这三种属性

