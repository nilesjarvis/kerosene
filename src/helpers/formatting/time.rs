pub fn format_duration(ms: u64) -> String {
    let mut secs = ms / 1000;
    if secs == 0 {
        return "< 1s".to_string();
    }

    let days = secs / 86400;
    secs %= 86400;
    let hours = secs / 3600;
    secs %= 3600;
    let mins = secs / 60;
    let s = secs % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if mins > 0 && days == 0 {
        parts.push(format!("{mins}m"));
    }
    if parts.is_empty() {
        parts.push(format!("{s}s"));
    }

    parts.join(" ")
}

pub fn format_timestamp_exact(unix_ms: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(unix_ms as i64)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%b %d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

pub fn format_relative_time(time_ms: u64, now_ms: u64) -> String {
    let diff_sec = now_ms.saturating_sub(time_ms) / 1000;
    if diff_sec < 60 {
        format!("{diff_sec}s")
    } else if diff_sec < 3600 {
        format!("{}m", diff_sec / 60)
    } else if diff_sec < 86400 {
        format!("{}h", diff_sec / 3600)
    } else {
        format!("{}d", diff_sec / 86400)
    }
}

pub fn format_timestamp(unix_secs: u64) -> String {
    let secs_per_day: u64 = 86400;
    let secs_per_hour: u64 = 3600;

    let total_days = unix_secs / secs_per_day;
    let remaining = unix_secs % secs_per_day;
    let hours = remaining / secs_per_hour;

    let mut y: u64 = 1970;
    let mut days_left = total_days;
    loop {
        let days_in_year =
            if y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)) {
                366
            } else {
                365
            };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        y += 1;
    }

    let is_leap = y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let month_days = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m: usize = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if days_left < md {
            m = i + 1;
            break;
        }
        days_left -= md;
    }
    let d = days_left + 1;

    format!("{m:02}/{d:02} {hours:02}:00")
}
