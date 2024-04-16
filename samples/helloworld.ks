
//mod D:\code\rs\tst\target\debug\tstlib.dll> m;

let a() {
  try throw (299)
  catch e {
    :e
  }
}

log(a())
