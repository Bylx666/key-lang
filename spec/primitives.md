## bool

唯一属性rev

true.rev == !true

方法rev, then

then会转达内部函数的返回值
false.rev().then(||:20) == 20

false.rev == false.rev() == true

## Func

唯一属性type, 值是Str: local extern native

## Buf

len ref capacity

## List

len capacity

## Str

len char_len(低性能) lines(低性能) capacity

