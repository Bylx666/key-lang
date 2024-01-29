## 定义结构

逗号可以省略

struct A {
  a: Type,
  b: Type,
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

## struct的储存

struct可以使用以下类型
struct Example {
  u8: Uint8
  u16: Uint16
  u32: Uint32
  u64: Uint   // Uint本来就是64位
  i8: Int8
  i16: Int16
  i32: Int32
  i64: Int
  f32: Float32
}

基本类型会按以下方式储存
Bool->  (u8)1/0
Int->   (i64) signed int
Uint->  (u64) unsigned int
Float-> (f64)double float
Str->   (LPSTR) string ptr 你要自己加\0
Buffer->(u64) buffer ptr 将str编码为utf16时或许会用到吧

以下类型需要特殊注意
]Func ->  (FARPROC) function ptr 待补充!
Array -> (u64) Array ptr 传进extern函数里是未定义行为
Obj   -> (u64) Obj ptr 不要传进extern函数里
struct-> (u64) Struct ptr 使用嵌套结构请手动展开，动态语言很难高性能处理嵌套
