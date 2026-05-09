use chrono::{Datelike, Duration, TimeZone, Utc};
use chrono_tz::Tz;

// ---------------------------------------------------------------------------
// Session-based starting points
// ---------------------------------------------------------------------------

/// Trading session for the normalization base time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    /// Most recent U.S. market open (9:30 AM ET).
    US,
    /// Most recent European market open (8:00 AM CET).
    Europe,
    /// Most recent Asian market open (9:00 AM JST = 0:00 UTC).
    Asia,
    /// Most recent UTC midnight.
    UtcDay,
    /// Most recent UTC week start (Monday 00:00 UTC).
    UtcWeek,
    /// Most recent UTC month start (day 1, 00:00 UTC).
    UtcMonth,
    /// Most recent UTC year start (Jan 1, 00:00 UTC).
    UtcYear,
}

impl Session {
    pub fn label(self) -> &'static str {
        match self {
            Session::US => "US",
            Session::Europe => "EU",
            Session::Asia => "Asia",
            Session::UtcDay => "UTC D",
            Session::UtcWeek => "UTC W",
            Session::UtcMonth => "UTC M",
            Session::UtcYear => "UTC Y",
        }
    }

    pub fn config_str(self) -> &'static str {
        match self {
            Session::US => "us",
            Session::Europe => "eu",
            Session::Asia => "asia",
            Session::UtcDay => "utc_day",
            Session::UtcWeek => "utc_week",
            Session::UtcMonth => "utc_month",
            Session::UtcYear => "utc_year",
        }
    }

    pub fn from_config_str(s: &str) -> Option<Self> {
        match s {
            "us" => Some(Session::US),
            "eu" => Some(Session::Europe),
            "asia" => Some(Session::Asia),
            "utc_day" => Some(Session::UtcDay),
            "utc_week" => Some(Session::UtcWeek),
            "utc_month" => Some(Session::UtcMonth),
            "utc_year" => Some(Session::UtcYear),
            _ => None,
        }
    }

    fn timezone(self) -> Tz {
        match self {
            Session::US => chrono_tz::America::New_York,
            Session::Europe => chrono_tz::Europe::London,
            Session::Asia => chrono_tz::Asia::Tokyo,
            Session::UtcDay | Session::UtcWeek | Session::UtcMonth | Session::UtcYear => {
                chrono_tz::UTC
            }
        }
    }

    fn open_hm(self) -> (u32, u32) {
        match self {
            Session::US => (9, 30),
            Session::Europe => (8, 0),
            Session::Asia => (9, 0),
            Session::UtcDay | Session::UtcWeek | Session::UtcMonth | Session::UtcYear => (0, 0),
        }
    }

    fn utc_anchor_ms(self, reference_utc: chrono::DateTime<Utc>) -> Option<u64> {
        match self {
            Session::UtcDay => u64::try_from(
                Utc.with_ymd_and_hms(
                    reference_utc.year(),
                    reference_utc.month(),
                    reference_utc.day(),
                    0,
                    0,
                    0,
                )
                .single()?
                .timestamp_millis(),
            )
            .ok(),
            Session::UtcWeek => {
                let days_since_monday = i64::from(reference_utc.weekday().num_days_from_monday());
                let monday = reference_utc.date_naive() - Duration::days(days_since_monday);
                u64::try_from(
                    Utc.with_ymd_and_hms(monday.year(), monday.month(), monday.day(), 0, 0, 0)
                        .single()?
                        .timestamp_millis(),
                )
                .ok()
            }
            Session::UtcMonth => u64::try_from(
                Utc.with_ymd_and_hms(reference_utc.year(), reference_utc.month(), 1, 0, 0, 0)
                    .single()?
                    .timestamp_millis(),
            )
            .ok(),
            Session::UtcYear => u64::try_from(
                Utc.with_ymd_and_hms(reference_utc.year(), 1, 1, 0, 0, 0)
                    .single()?
                    .timestamp_millis(),
            )
            .ok(),
            Session::US | Session::Europe | Session::Asia => None,
        }
    }

    fn open_utc_ms_for_local_date(self, date: chrono::NaiveDate) -> Option<u64> {
        let (hour, minute) = self.open_hm();
        let naive = date.and_hms_opt(hour, minute, 0)?;
        let tz = self.timezone();
        let local = tz
            .from_local_datetime(&naive)
            .earliest()
            .or_else(|| tz.from_local_datetime(&naive).latest())?;
        u64::try_from(local.with_timezone(&Utc).timestamp_millis()).ok()
    }

    /// Compute the most recent session open timestamp (in ms) before
    /// `reference_ms`, using timezone-aware equity session opens.
    pub fn last_open_ms(self, reference_ms: u64) -> u64 {
        let Ok(reference_i64) = i64::try_from(reference_ms) else {
            return reference_ms;
        };
        let Some(reference_utc) = Utc.timestamp_millis_opt(reference_i64).single() else {
            return reference_ms;
        };

        if let Some(anchor) = self.utc_anchor_ms(reference_utc) {
            return anchor;
        }

        let reference_local = reference_utc.with_timezone(&self.timezone());
        let today_date = reference_local.date_naive();
        let today_open = self
            .open_utc_ms_for_local_date(today_date)
            .unwrap_or(reference_ms);

        if reference_ms >= today_open {
            return today_open;
        }

        let previous_date = today_date - Duration::days(1);
        self.open_utc_ms_for_local_date(previous_date)
            .unwrap_or_else(|| today_open.saturating_sub(86_400_000))
    }
}

pub const SESSION_OPTIONS: &[Session] = &[
    Session::US,
    Session::Europe,
    Session::Asia,
    Session::UtcDay,
    Session::UtcWeek,
    Session::UtcMonth,
    Session::UtcYear,
];
