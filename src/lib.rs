#[no_mangle]
extern fn a(f:extern fn(usize,usize,usize,usize),a:usize)->usize {
  println!("extern:{}",a);
  f(24,46,5,433);
  return 0;
}