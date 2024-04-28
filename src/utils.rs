//! 一些很通用的函数

/// 格式化当前时间
#[inline]
pub fn date() -> String {
    #[inline]
    fn is_leap(year: u64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 4 == 400
    }
    let t = std::time::SystemTime::now();
    let mut t = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let sec = t % 60;
    t /= 60;
    let min = t % 60;
    t /= 60;
    let hour = t % 24 + 8;
    t /= 24;
    let mut year = 1970;
    while t >= 365 {
        t -= 365;
        if is_leap(year) {
            if t >= 1 {
                t -= 1
            } else {
                break;
            }
        }
        year += 1;
    }

    let mut month_list: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_leap(year) {
        month_list[1] = 29
    };

    let mut mon: u8 = 1;
    for n in month_list {
        if t >= n {
            t -= n;
        } else {
            break;
        }
        mon += 1;
    }
    format!("{year}/{mon:02}/{t:02} {hour:02}:{min:02}:{sec:02}")
}

#[inline]
pub fn to_absolute_path(s: String) -> String {
    let p = std::path::Path::new(&s);
    if p.is_absolute() {
        s
    } else {
        let mut buf = std::path::PathBuf::new();
        buf.push(std::env::current_dir().expect("无法获取当前文件夹, 请尝试传入绝对路径"));
        buf.push(s);
        buf.into_os_string().to_string_lossy().into_owned()
    }
}
