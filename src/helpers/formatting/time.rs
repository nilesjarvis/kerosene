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

pub fn format_precise_duration(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        format!("{duration_ms} ms")
    } else if duration_ms < 60_000 {
        format!("{}.{:03} s", duration_ms / 1_000, duration_ms % 1_000)
    } else if duration_ms < 3_600_000 {
        format!(
            "{}m {}s",
            duration_ms / 60_000,
            (duration_ms % 60_000) / 1_000
        )
    } else {
        format!(
            "{}h {}m",
            duration_ms / 3_600_000,
            (duration_ms % 3_600_000) / 60_000
        )
    }
}

pub fn format_seen_latency_label(
    timestamp_ms: u64,
    fetched_at_ms: u64,
    first_seen_ms: u64,
) -> Option<String> {
    if fetched_at_ms == 0 || first_seen_ms == 0 {
        None
    } else {
        Some(format!(
            "seen +{}",
            format_precise_duration(fetched_at_ms.saturating_sub(timestamp_ms))
        ))
    }
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
