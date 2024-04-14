
//mod D:\code\rs\tst\target\debug\tstlib.dll> m;

// 该函数直接返回元素的type属性
// 如果是没有type属性的值, 
// 返回值自然就是uninit而不是Str
// 因此就会被自动跳过
let group_func(v): v.type;

log(Obj::group_by([
  {
    name: "芙卡洛斯"
    type: "神"
  }
  {
    name: "若陀"
    type: "龙"
  }
  "一个不是对象的异类",
  {
    name: "那位来客"
    type: "龙"
  }
], group_func))

