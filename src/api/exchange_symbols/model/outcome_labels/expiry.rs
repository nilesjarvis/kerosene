use super::OutcomeSymbolInfo;
use crate::helpers;

impl OutcomeSymbolInfo {
    pub(super) fn format_expiry_at(expiry: &str, now_ms: Option<u64>) -> String {
        chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
            .map(|dt| {
                let expiry_label = dt.format("%Y-%m-%d %H:%M UTC").to_string();
                now_ms
                    .and_then(|now_ms| expiry_countdown_label(expiry, now_ms))
                    .map(|countdown| format!("{expiry_label} ({countdown})"))
                    .unwrap_or(expiry_label)
            })
            .unwrap_or_else(|_| expiry.to_string())
    }
}

fn expiry_countdown_label(expiry: &str, now_ms: u64) -> Option<String> {
    let expiry_ms = chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
        .ok()?
        .and_utc()
        .timestamp_millis();
    let now_ms = i64::try_from(now_ms).ok()?;
    let diff_ms = expiry_ms.saturating_sub(now_ms);
    if diff_ms <= 0 {
        return Some("expired".to_string());
    }

    Some(format!("{} left", helpers::format_duration(diff_ms as u64)))
}
