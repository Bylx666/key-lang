程序并不保证你的指针是否正确。

let a = 0u;
let p:Uint = Mem::ptr(a); // ptr to a
let p:Uint = Mem::alloc(20); // ptr to allocator

let p = 0;
p = 0;

```
Mem::sizeof(Uint); // You cannot bring Types into any other funcs!
let a:Uint = Mem::read(p); // type is nessasery
Mem::write(p, n);
Mem::leak(p);
Mem::drop(p);
Mem::typeof(a);
Mem::alloc(20);
```

## transmutation
```
let a = 0u;
let p = Mem::ptr(a);
let f:0i = Mem::read(a);
```
why not `a.into_int()`?