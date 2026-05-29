/// 返回系统本地时间的 ISO 格式字符串
///
/// 使用 libc 的 `localtime_r`（Unix）或 `localtime_s`（Windows）获取本地时区时间，
/// 避免引入 chrono 等重量级时间库。
///
/// # 返回值
/// 格式为 `YYYY-MM-DDTHH:MM:SS` 的 19 字符字符串，例如 `"2025-05-29T14:30:00"`
///
/// # Panics
/// 当 `SystemTime::now()` 早于 UNIX_EPOCH 时会 panic（正常系统不会发生）
pub fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let timestamp = now.as_secs() as libc::time_t;

    // SAFETY: zeroed() 对 libc::tm 是安全的，该结构体所有字段均为整数类型，
    // 全零是合法的初始状态，后续会被 localtime_r/localtime_s 完整覆写。
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };

    #[cfg(unix)]
    // SAFETY: timestamp 是有效的 time_t 值，tm 是已分配的可变引用，
    // localtime_r 是线程安全的（不使用全局静态缓冲区），会将结果写入 tm。
    unsafe {
        libc::localtime_r(&timestamp, &mut tm);
    }

    #[cfg(windows)]
    // SAFETY: timestamp 是有效的 time_t 值，tm 是已分配的可变引用，
    // localtime_s 是 Windows 上的线程安全版本，会将结果写入 tm。
    unsafe {
        libc::localtime_s(&mut tm, &timestamp);
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrono_now_format() {
        let now = chrono_now();
        // 格式: YYYY-MM-DDTHH:MM:SS
        assert_eq!(now.len(), 19);
        assert_eq!(&now[4..5], "-");
        assert_eq!(&now[7..8], "-");
        assert_eq!(&now[10..11], "T");
        assert_eq!(&now[13..14], ":");
        assert_eq!(&now[16..17], ":");
    }

    #[test]
    fn test_chrono_now_valid_date() {
        let now = chrono_now();
        let year: u32 = now[0..4].parse().unwrap();
        let month: u32 = now[5..7].parse().unwrap();
        let day: u32 = now[8..10].parse().unwrap();
        let hour: u32 = now[11..13].parse().unwrap();
        let minute: u32 = now[14..16].parse().unwrap();
        let second: u32 = now[17..19].parse().unwrap();

        assert!(year >= 2024);
        assert!((1..=12).contains(&month));
        assert!((1..=31).contains(&day));
        assert!(hour < 24);
        assert!(minute < 60);
        assert!(second < 60);
    }
}
