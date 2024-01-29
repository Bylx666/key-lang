mod D:\code\rs\key-native\target\debug\key_native.dll> mymod;

mymod-.test();

"
extern kernel32> {
  CreateThread(a,b,c,d,e,f)
  WaitForSingleObject(a,b)
}
let f(a) {
  print(a)
}
WaitForSingleObject(CreateThread(0,0,f,22,0,0),99999)
"