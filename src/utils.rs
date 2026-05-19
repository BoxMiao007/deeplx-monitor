/// 返回系统本地时间的 ISO 格式字符串
pub fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let timestamp = now.as_secs() as libc::time_t;

    let mut tm: libc::tm = unsafe { std::mem::zeroed() };

    #[cfg(unix)]
    unsafe { libc::localtime_r(&timestamp, &mut tm); }

    #[cfg(windows)]
    unsafe { libc::localtime_s(&mut tm, &timestamp); }

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
