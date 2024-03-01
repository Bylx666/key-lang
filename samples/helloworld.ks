
//mod D:\code\rs\key-native\target\debug\key_native.dll> m
mod D:\code\rs\key-lang\samples\testmod.ks> m;

let a(m) {
  :m
}
let o = {
  a:a, b:a
}
let t = m-:Sample::new()
t.a = o
log(t.a.a(200))
