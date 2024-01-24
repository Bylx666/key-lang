
extern kernel32> {
  std: GetStdHandle(n:Int)
  write: WriteConsoleA(a,b,c,d,e)
}

let h = std(-11)
write(h, "word", 4)
