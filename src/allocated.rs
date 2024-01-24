//! 内存控制

#[inline]
pub fn leak<T>(v:T)-> *mut T {
  Box::into_raw(Box::new(v))
}