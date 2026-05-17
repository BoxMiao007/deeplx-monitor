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
