//! 运行时提供的基础api
//! 
//! 和对基本类型方法的实现

use crate::intern::Interned;

pub mod std;

pub mod obj;

// pub fn find_impl(left:Interned, right:Interned) {
//   match left.vec() {
//     b"Obj"=> obj::
//   }
// }
