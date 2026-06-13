use crate::app_state::TradingTerminal;
use chrono::{DateTime, Local, Utc};

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

pub(crate) fn cooldown_heat(first_seen_ms: u64, now_ms: u64, cooldown_ms: u64) -> f32 {
    if first_seen_ms == 0 {
        return 0.0;
    }

    let age_ms = now_ms.saturating_sub(first_seen_ms);
    if age_ms >= cooldown_ms {
        0.0
    } else {
        1.0 - (age_ms as f32 / cooldown_ms as f32)
    }
}

pub(crate) fn utc_datetime_from_unix_ms(unix_ms: u64) -> DateTime<Utc> {
    let unix_ms = i64::try_from(unix_ms).unwrap_or(i64::MAX);
    DateTime::<Utc>::from_timestamp_millis(unix_ms)
        .unwrap_or_else(|| DateTime::<Utc>::from(std::time::UNIX_EPOCH))
}

pub(crate) fn local_datetime_from_unix_ms(unix_ms: u64) -> DateTime<Local> {
    utc_datetime_from_unix_ms(unix_ms).with_timezone(&Local)
}

impl TradingTerminal {
    pub(crate) fn now_ms() -> u64 {
        now_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::{local_datetime_from_unix_ms, utc_datetime_from_unix_ms};
    use chrono::{TimeZone, Utc};

    #[test]
    fn datetime_from_unix_ms_uses_supplied_timestamp() {
        let expected_utc = Utc
            .with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
            .single()
            .expect("valid UTC timestamp");

        let utc = utc_datetime_from_unix_ms(expected_utc.timestamp_millis() as u64);
        let local = local_datetime_from_unix_ms(expected_utc.timestamp_millis() as u64);

        assert_eq!(utc, expected_utc);
        assert_eq!(local.timestamp_millis(), expected_utc.timestamp_millis());
    }

    #[test]
    fn datetime_from_unix_ms_falls_back_without_sampling_clock() {
        let unix_epoch = Utc
            .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
            .single()
            .expect("valid UTC timestamp");

        assert_eq!(utc_datetime_from_unix_ms(u64::MAX), unix_epoch);
    }
}
