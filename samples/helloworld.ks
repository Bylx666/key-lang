
//mod D:\code\rs\key-native\target\debug\key_native.dll> m
//mod D:\code\rs\key-lang\samples\testmod.ks> m;

log(Obj::group_by([
  {name:"伞兵",id:2,type:"god"},
  {name:"芙卡洛斯",id:3,type:"god"},
  {name:"宵宫",id:4,type:"human"},
  {name:"千织",id:5}
], |e|:e.type));