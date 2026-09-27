use super::OutcomeSymbolInfo;
use crate::helpers;

impl OutcomeSymbolInfo {
    pub(super) fn label_with_expiry(
        label: String,
        expiry: Option<&str>,
        now_ms: Option<u64>,
        include_expiry: bool,
    ) -> String {
        if include_expiry && let Some(expiry) = expiry {
            format!("{label} at {}", Self::format_expiry_at(expiry, now_ms))
        } else {
            label
        }
    }

    pub fn time_left_label(&self, now_ms: u64) -> Option<String> {
        self.question_expiry
            .as_deref()
            .or(self.expiry.as_deref())
            .and_then(|expiry| expiry_time_left_label(expiry, now_ms))
    }

    pub(super) fn format_expiry_at(expiry: &str, now_ms: Option<u64>) -> String {
        chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
            .map(|dt| {
                let expiry_label = dt.format("%Y-%m-%d %H:%M UTC").to_string();
                now_ms
                    .and_then(|now_ms| time_left_label_at(dt.and_utc().timestamp_millis(), now_ms))
                    .map(|countdown| {
                        if countdown == "expired" {
                            format!("{expiry_label} ({countdown})")
                        } else {
                            format!("{expiry_label} ({countdown} left)")
                        }
                    })
                    .unwrap_or(expiry_label)
            })
            .unwrap_or_else(|_| expiry.to_string())
    }
}

fn expiry_time_left_label(expiry: &str, now_ms: u64) -> Option<String> {
    let expiry_ms = chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
        .ok()?
        .and_utc()
        .timestamp_millis();
    time_left_label_at(expiry_ms, now_ms)
}

fn time_left_label_at(expiry_ms: i64, now_ms: u64) -> Option<String> {
    let now_ms = i64::try_from(now_ms).ok()?;
    let diff_ms = expiry_ms.saturating_sub(now_ms);
    if diff_ms <= 0 {
        return Some("expired".to_string());
    }

    Some(helpers::format_duration(diff_ms as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp_ms(expiry: &str) -> u64 {
        chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
            .expect("valid expiry")
            .and_utc()
            .timestamp_millis() as u64
    }

    #[test]
    fn expiry_time_left_label_formats_countdown_and_expired_values() {
        let expiry = "20260520-0600";
        let now_ms = timestamp_ms(expiry).saturating_sub(3_660_000);

        assert_eq!(
            expiry_time_left_label(expiry, now_ms),
            Some("1h 1m".to_string())
        );
        assert_eq!(
            OutcomeSymbolInfo::format_expiry_at(expiry, Some(now_ms)),
            "2026-05-20 06:00 UTC (1h 1m left)"
        );
        assert_eq!(
            expiry_time_left_label(expiry, timestamp_ms(expiry)),
            Some("expired".to_string())
        );
        assert_eq!(expiry_time_left_label("bad", now_ms), None);
    }
}
