use crate::app_state::TradingTerminal;
use chrono::{DateTime, Utc};
use chrono_tz::{Africa, America, Asia, Australia, Europe, Pacific, Tz};
use serde::{Deserialize, Serialize};

/// Global wall-clock display preference. Persist the IANA identifier so the
/// offset is resolved for each instant, including historical DST transitions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(from = "String", into = "String")]
pub struct DisplayTimeZone(pub(crate) Tz);

impl DisplayTimeZone {
    pub(crate) const CHOICES: &'static [Self] = &[
        Self(Tz::UTC),
        Self(Australia::Adelaide),
        Self(Pacific::Auckland),
        Self(Asia::Bangkok),
        Self(Europe::Berlin),
        Self(America::Argentina::Buenos_Aires),
        Self(Africa::Cairo),
        Self(America::Chicago),
        Self(Asia::Colombo),
        Self(America::Denver),
        Self(Asia::Dubai),
        Self(Asia::Hong_Kong),
        Self(Pacific::Honolulu),
        Self(Europe::Istanbul),
        Self(Africa::Johannesburg),
        Self(Asia::Karachi),
        Self(Asia::Kathmandu),
        Self(Asia::Kolkata),
        Self(Africa::Lagos),
        Self(Europe::London),
        Self(America::Los_Angeles),
        Self(America::Mexico_City),
        Self(Europe::Moscow),
        Self(America::New_York),
        Self(Europe::Paris),
        Self(America::Phoenix),
        Self(America::Santiago),
        Self(America::Sao_Paulo),
        Self(Asia::Seoul),
        Self(Asia::Shanghai),
        Self(Asia::Singapore),
        Self(Australia::Sydney),
        Self(Asia::Tokyo),
        Self(America::Toronto),
        Self(Europe::Zurich),
    ];

    pub(crate) fn datetime(self, ms: u64) -> Option<DateTime<Tz>> {
        DateTime::from_timestamp_millis(i64::try_from(ms).ok()?)
            .map(|utc| utc.with_timezone(&self.0))
    }

    pub(crate) fn format(self, ms: u64, pattern: &str, fallback: &str) -> String {
        self.datetime(ms)
            .map(|dt| dt.format(pattern).to_string())
            .unwrap_or_else(|| fallback.to_owned())
    }

    pub(crate) fn format_rfc3339(self, value: &str, pattern: &str) -> String {
        DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&self.0).format(pattern).to_string())
            .unwrap_or_else(|_| value.to_owned())
    }

    pub(crate) fn format_timestamp(self, ms: u64) -> String {
        self.format(ms, "%b %d, %H:%M", "Unknown")
    }
}

impl std::fmt::Display for DisplayTimeZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.0.name();
        let city = id.rsplit('/').next().unwrap_or(id);
        f.write_str(&city.replace('_', " "))
    }
}

impl From<String> for DisplayTimeZone {
    fn from(id: String) -> Self {
        Self(id.parse().unwrap_or_default())
    }
}

impl From<DisplayTimeZone> for String {
    fn from(zone: DisplayTimeZone) -> Self {
        zone.0.name().to_owned()
    }
}

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

impl TradingTerminal {
    pub(crate) fn now_ms() -> u64 {
        now_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::utc_datetime_from_unix_ms;
    use chrono::{TimeZone, Utc};

    #[test]
    fn datetime_from_unix_ms_uses_supplied_timestamp() {
        let expected_utc = Utc
            .with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
            .single()
            .expect("valid UTC timestamp");

        let utc = utc_datetime_from_unix_ms(expected_utc.timestamp_millis() as u64);

        assert_eq!(utc, expected_utc);
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
