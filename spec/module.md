# 模块化

文件名后的箭头前不可空格

mod test.dll> my_mod
mod test.ksm> my_mod
mod C:\d\e.ksm> my_mod

mod test.ks> my_mod

需要注意的是，Key并不会缓存你的模块。如果你在一个程序中对同一文件使用了两次mod语句，Key会执行两次对应的程序并得到两个独立的模块。
同样的，每个ks模块中引用的模块也都分别独立，这样的话对同一个模块的上下文就不会因为多个模块对其的修改行为而乱套。

只能导出函数和struct

## 原生模块 Native Mods
