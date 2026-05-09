use chrono::{DateTime, Utc};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Account History Formatting
// ---------------------------------------------------------------------------

pub(super) fn format_history_time_millis(ms: u64) -> String {
    let Ok(ms) = i64::try_from(ms) else {
        return "--/-- --:--".to_string();
    };

    DateTime::<Utc>::from_timestamp_millis(ms)
        .map(|dt| dt.format("%m/%d %H:%M").to_string())
        .unwrap_or_else(|| "--/-- --:--".to_string())
}
