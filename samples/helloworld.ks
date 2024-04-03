
//mod samples/testmod.ks> m;

extern kernel32> {
  std: GetStdHandle(n:Int)
  write: WriteConsoleW(
    output:HANDLE,
    buffer:LPCVOID,
    charNum:DWORD,
    written:LPDWORD,
    rev
  )
}

let s = "原神, 启动!";
write(std(-11), s.to_utf16(), s.len)
