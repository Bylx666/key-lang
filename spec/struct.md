## 定义结构

struct A {
  a: Type
  b: Type
}

## 定义方法


struct B {
  a: Type
  b: Type
}
impl B {
  new(): B {
    B {}
  }
  method(self, b:Uint) {

  }
}

## 别名

key A = B;

## obj

希望能和js的Object玩起来手感差不多

let obj = {
  a
  b: 24
}

## c struct

模拟C的struct，支持传入函数

cstruct ClassName {
  a:Uint8,
  b:Buffer(u16),
  c:Str
}

仅允许以下类型
cstruct Uint8 Uint16 Uint32 Int8 Int16 Int32 Float32
基本类型会按以下方式储存
Bool->  (u8)1/0
Int->   (i64) signed int
Uint->  (u64) unsigned int
Float-> (f64)double float
Func->  (FARPROC) function ptr
Str->   (LPSTR) string ptr 你要自己加\0
Buffer->(u64) buffer ptr 将str编码为utf16时或许会用到吧
Array-> 报错
