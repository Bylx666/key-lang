
mod D:\code\rs\tst\target\debug\tstlib.dll> m;
let delay = m-.async_delay;

// 未使用fall, 因此不会阻塞
log(delay(233, 2000)); // 打印 Planet { Builtin }

// 使用fall
log(delay(233, 1000).fall()); // 1秒后打印233

// 由于未使用wait_inc, 第一个delay还未完成时程序就退出了
