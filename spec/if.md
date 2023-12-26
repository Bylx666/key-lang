
if value {

}
else {

}

if value return n;
else return n;
if value break n;
else break n;
if value continue;
else continue;

Key语言不使用!,&和|作为条件判断符号，而是使用not, and, or关键词替代。

let a = cd1==2 and c2 or c3
if not a return;

Key语言使用0b作为false，1b作为true，这意味着你可以自定义这两个词的具体含义了。不喜欢的话写一句`const true = 1b;`就可以了。
