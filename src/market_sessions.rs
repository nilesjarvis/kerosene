use crate::helpers::not_available_placeholder;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;

// ---------------------------------------------------------------------------
// Market Session Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MarketSession {
    NewYork,
    Overnight,
    Asia,
    London,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarketSessionRange {
    pub(crate) kind: MarketSession,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionAnchor {
    Market(MarketSession),
    UtcDay,
    UtcWeek,
    UtcMonth,
    UtcYear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarketSessionBoundary {
    time_ms: u64,
    next_kind: MarketSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarketClockSpec {
    label: &'static str,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
}

pub(crate) const MARKET_CLOCK_SESSIONS: [MarketSession; 3] = [
    MarketSession::London,
    MarketSession::Asia,
    MarketSession::NewYork,
];

impl MarketSession {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NewYork => "New York",
            Self::Overnight => "Overnight",
            Self::Asia => "Asia",
            Self::London => "London",
        }
    }

    pub(crate) fn short_label(self) -> &'static str {
        match self {
            Self::NewYork => "NY",
            Self::Overnight => "O/N",
            Self::Asia => "Asia",
            Self::London => "London",
        }
    }

    pub(crate) fn timezone(self) -> Tz {
        match self {
            Self::NewYork | Self::Overnight => chrono_tz::America::New_York,
            Self::Asia => chrono_tz::Asia::Tokyo,
            Self::London => chrono_tz::Europe::London,
        }
    }

    pub(crate) fn open_hm(self) -> (u32, u32) {
        match self {
            Self::NewYork => (9, 30),
            Self::Overnight => (16, 0),
            Self::Asia => (9, 0),
            Self::London => (8, 0),
        }
    }

    pub(crate) fn close_hm(self) -> Option<(u32, u32)> {
        match self {
            Self::NewYork => Some((16, 0)),
            Self::Asia => Some((15, 0)),
            Self::London => Some((16, 30)),
            Self::Overnight => None,
        }
    }

    pub(crate) fn open_utc_ms_for_local_date(self, date: chrono::NaiveDate) -> Option<u64> {
        let (hour, minute) = self.open_hm();
        local_time_ms(self.timezone(), date, hour, minute)
    }

    pub(crate) fn last_open_ms(self, reference_ms: u64) -> u64 {
        let Ok(reference_i64) = i64::try_from(reference_ms) else {
            return reference_ms;
        };
        let Some(reference_utc) = Utc.timestamp_millis_opt(reference_i64).single() else {
            return reference_ms;
        };

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

    pub(crate) fn market_is_active(self, now_utc: DateTime<Utc>) -> bool {
        self.clock_spec()
            .is_some_and(|spec| market_window_is_active(now_utc, spec.tz, spec.open, spec.close))
    }

    pub(crate) fn market_clock_text(self, now_utc: DateTime<Utc>) -> Option<String> {
        let spec = self.clock_spec()?;
        Some(market_clock_text_for_window(
            spec.label, now_utc, spec.tz, spec.open, spec.close,
        ))
    }

    fn clock_spec(self) -> Option<MarketClockSpec> {
        Some(MarketClockSpec {
            label: self.label(),
            tz: self.timezone(),
            open: self.open_hm(),
            close: self.close_hm()?,
        })
    }
}

impl SessionAnchor {
    pub(crate) fn last_open_ms(self, reference_ms: u64) -> u64 {
        let Ok(reference_i64) = i64::try_from(reference_ms) else {
            return reference_ms;
        };
        let Some(reference_utc) = Utc.timestamp_millis_opt(reference_i64).single() else {
            return reference_ms;
        };

        let anchor = match self {
            Self::Market(session) => return session.last_open_ms(reference_ms),
            Self::UtcDay => utc_boundary_ms(
                reference_utc.year(),
                reference_utc.month(),
                reference_utc.day(),
            ),
            Self::UtcWeek => {
                let days_since_monday = i64::from(reference_utc.weekday().num_days_from_monday());
                let monday = reference_utc.date_naive() - Duration::days(days_since_monday);
                utc_boundary_ms(monday.year(), monday.month(), monday.day())
            }
            Self::UtcMonth => utc_boundary_ms(reference_utc.year(), reference_utc.month(), 1),
            Self::UtcYear => utc_boundary_ms(reference_utc.year(), 1, 1),
        };
        anchor.unwrap_or(reference_ms)
    }
}

pub(crate) fn visible_session_ranges(start_ms: u64, end_ms: u64) -> Vec<MarketSessionRange> {
    if end_ms <= start_ms {
        return Vec::new();
    }

    let boundaries = session_boundaries(start_ms, end_ms);
    boundaries
        .windows(2)
        .filter_map(|pair| {
            let start = pair[0].time_ms;
            let end = pair[1].time_ms;
            (end > start_ms && start < end_ms && end > start).then_some(MarketSessionRange {
                kind: pair[0].next_kind,
                start_ms: start,
                end_ms: end,
            })
        })
        .collect()
}

pub(crate) fn market_window_is_active(
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> bool {
    let local_now = now_utc.with_timezone(&tz);
    let date = local_now.date_naive();
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }

    let Some(open_local) = local_datetime_for_date(tz, date, open.0, open.1) else {
        return false;
    };
    let Some(close_local) = local_datetime_for_date(tz, date, close.0, close.1) else {
        return false;
    };

    local_now >= open_local && local_now < close_local
}

pub(crate) fn market_clock_text_for_window(
    label: &str,
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> String {
    let now_local = now_utc.with_timezone(&tz);
    let session_status = if market_window_is_active(now_utc, tz, open, close) {
        let close_in = current_market_close(now_utc, tz, close.0, close.1)
            .map(|dt| format_market_time_until(now_utc, dt.with_timezone(&Utc)))
            .unwrap_or_else(not_available_placeholder);
        format!("closes in {close_in}")
    } else {
        let next_open = next_market_open(now_utc, tz, open.0, open.1)
            .map(|dt| format_market_time_until(now_utc, dt.with_timezone(&Utc)))
            .unwrap_or_else(not_available_placeholder);
        format!("opens in {next_open}")
    };
    format!(
        "{label} {:02}:{:02}:{:02} {} ({session_status})",
        now_local.hour(),
        now_local.minute(),
        now_local.second(),
        now_local.format("%Z")
    )
}

fn session_boundaries(start_ms: u64, end_ms: u64) -> Vec<MarketSessionBoundary> {
    let Some(start_dt) = utc_from_ms(start_ms) else {
        return Vec::new();
    };
    let Some(end_dt) = utc_from_ms(end_ms) else {
        return Vec::new();
    };

    let mut date = start_dt.date_naive() - Duration::days(3);
    let last_date = end_dt.date_naive() + Duration::days(3);
    let mut boundaries = Vec::new();

    while date <= last_date {
        for session in [
            MarketSession::Asia,
            MarketSession::London,
            MarketSession::NewYork,
            MarketSession::Overnight,
        ] {
            if let Some(time_ms) = session.open_utc_ms_for_local_date(date) {
                boundaries.push(MarketSessionBoundary {
                    time_ms,
                    next_kind: session,
                });
            }
        }
        date += Duration::days(1);
    }

    boundaries.sort_by_key(|boundary| boundary.time_ms);
    boundaries.dedup_by_key(|boundary| boundary.time_ms);
    boundaries
}

fn format_market_time_until(now_utc: DateTime<Utc>, target_utc: DateTime<Utc>) -> String {
    let diff = target_utc - now_utc;
    let total_minutes = diff.num_minutes().max(0);
    let days = total_minutes / (24 * 60);
    let hours = (total_minutes % (24 * 60)) / 60;
    let minutes = total_minutes % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else {
        format!("{hours}h {minutes}m")
    }
}

fn next_market_open(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    for day_offset in 0..8 {
        let date = local_now.date_naive() + Duration::days(day_offset);
        if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            continue;
        }
        let open_local = local_datetime_for_date(tz, date, hour, minute)?;
        if open_local > local_now {
            return Some(open_local);
        }
    }
    None
}

fn current_market_close(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    let close_local = local_datetime_for_date(tz, local_now.date_naive(), hour, minute)?;
    (close_local > local_now).then_some(close_local)
}

fn local_time_ms(tz: Tz, date: chrono::NaiveDate, hour: u32, minute: u32) -> Option<u64> {
    local_datetime_for_date(tz, date, hour, minute)
        .and_then(|local| u64::try_from(local.with_timezone(&Utc).timestamp_millis()).ok())
}

fn local_datetime_for_date(
    tz: Tz,
    date: chrono::NaiveDate,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    tz.from_local_datetime(&naive)
        .earliest()
        .or_else(|| tz.from_local_datetime(&naive).latest())
}

fn utc_boundary_ms(year: i32, month: u32, day: u32) -> Option<u64> {
    u64::try_from(
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()?
            .timestamp_millis(),
    )
    .ok()
}

fn utc_from_ms(time_ms: u64) -> Option<chrono::DateTime<Utc>> {
    let time_ms = i64::try_from(time_ms).ok()?;
    Utc.timestamp_millis_opt(time_ms).single()
}

#[cfg(test)]
mod tests {
    use super::{
        MarketSession, SessionAnchor, market_clock_text_for_window, market_window_is_active,
        visible_session_ranges,
    };
    use chrono::{TimeZone, Utc};

    fn ts(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
        u64::try_from(
            Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
                .single()
                .expect("valid UTC timestamp")
                .timestamp_millis(),
        )
        .expect("positive timestamp")
    }

    #[test]
    fn session_ranges_follow_winter_market_offsets() {
        let ranges = visible_session_ranges(ts(2026, 1, 14, 0, 0), ts(2026, 1, 15, 1, 0));
        let visible: Vec<_> = ranges
            .iter()
            .filter(|range| {
                range.start_ms >= ts(2026, 1, 14, 0, 0) && range.start_ms < ts(2026, 1, 15, 0, 0)
            })
            .map(|range| (range.kind, range.start_ms, range.end_ms))
            .collect();

        assert_eq!(
            visible,
            vec![
                (
                    MarketSession::Asia,
                    ts(2026, 1, 14, 0, 0),
                    ts(2026, 1, 14, 8, 0),
                ),
                (
                    MarketSession::London,
                    ts(2026, 1, 14, 8, 0),
                    ts(2026, 1, 14, 14, 30),
                ),
                (
                    MarketSession::NewYork,
                    ts(2026, 1, 14, 14, 30),
                    ts(2026, 1, 14, 21, 0),
                ),
                (
                    MarketSession::Overnight,
                    ts(2026, 1, 14, 21, 0),
                    ts(2026, 1, 15, 0, 0),
                ),
            ]
        );
    }

    #[test]
    fn session_ranges_follow_summer_market_offsets() {
        let ranges = visible_session_ranges(ts(2026, 7, 14, 0, 0), ts(2026, 7, 15, 1, 0));
        let visible: Vec<_> = ranges
            .iter()
            .filter(|range| {
                range.start_ms >= ts(2026, 7, 14, 0, 0) && range.start_ms < ts(2026, 7, 15, 0, 0)
            })
            .map(|range| (range.kind, range.start_ms, range.end_ms))
            .collect();

        assert_eq!(
            visible,
            vec![
                (
                    MarketSession::Asia,
                    ts(2026, 7, 14, 0, 0),
                    ts(2026, 7, 14, 7, 0),
                ),
                (
                    MarketSession::London,
                    ts(2026, 7, 14, 7, 0),
                    ts(2026, 7, 14, 13, 30),
                ),
                (
                    MarketSession::NewYork,
                    ts(2026, 7, 14, 13, 30),
                    ts(2026, 7, 14, 20, 0),
                ),
                (
                    MarketSession::Overnight,
                    ts(2026, 7, 14, 20, 0),
                    ts(2026, 7, 15, 0, 0),
                ),
            ]
        );
    }

    #[test]
    fn session_anchors_handle_regional_dst_offsets() {
        assert_eq!(
            SessionAnchor::Market(MarketSession::NewYork).last_open_ms(ts(2026, 7, 14, 16, 0)),
            ts(2026, 7, 14, 13, 30)
        );
        assert_eq!(
            SessionAnchor::Market(MarketSession::NewYork).last_open_ms(ts(2026, 1, 14, 16, 0)),
            ts(2026, 1, 14, 14, 30)
        );
        assert_eq!(
            SessionAnchor::Market(MarketSession::London).last_open_ms(ts(2026, 7, 14, 10, 0)),
            ts(2026, 7, 14, 7, 0)
        );
        assert_eq!(
            SessionAnchor::Market(MarketSession::London).last_open_ms(ts(2026, 1, 14, 10, 0)),
            ts(2026, 1, 14, 8, 0)
        );
    }

    #[test]
    fn utc_anchors_use_calendar_boundaries() {
        assert_eq!(
            SessionAnchor::UtcDay.last_open_ms(ts(2026, 3, 28, 15, 42)),
            ts(2026, 3, 28, 0, 0)
        );
        assert_eq!(
            SessionAnchor::UtcWeek.last_open_ms(ts(2026, 3, 28, 11, 0)),
            ts(2026, 3, 23, 0, 0)
        );
        assert_eq!(
            SessionAnchor::UtcMonth.last_open_ms(ts(2026, 3, 28, 15, 42)),
            ts(2026, 3, 1, 0, 0)
        );
        assert_eq!(
            SessionAnchor::UtcYear.last_open_ms(ts(2026, 8, 14, 2, 30)),
            ts(2026, 1, 1, 0, 0)
        );
    }

    #[test]
    fn market_activity_uses_local_hours_and_weekends() {
        assert!(
            MarketSession::NewYork.market_is_active(
                Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
                    .single()
                    .expect("valid UTC timestamp")
            )
        );
        assert!(
            !MarketSession::NewYork.market_is_active(
                Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0)
                    .single()
                    .expect("valid UTC timestamp")
            )
        );
        assert!(
            !MarketSession::NewYork.market_is_active(
                Utc.with_ymd_and_hms(2026, 5, 16, 14, 0, 0)
                    .single()
                    .expect("valid UTC timestamp")
            )
        );
        assert!(market_window_is_active(
            Utc.with_ymd_and_hms(2026, 5, 18, 1, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::Asia::Tokyo,
            (9, 0),
            (15, 0),
        ));
    }

    #[test]
    fn market_clock_text_reports_open_and_close_countdowns() {
        let active = MarketSession::NewYork
            .market_clock_text(
                Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
                    .single()
                    .expect("valid UTC timestamp"),
            )
            .expect("clock text");
        assert_eq!(active, "New York 10:00:00 EDT (closes in 6h 0m)");

        let inactive = market_clock_text_for_window(
            "New York",
            Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0)
                .single()
                .expect("valid UTC timestamp"),
            chrono_tz::America::New_York,
            (9, 30),
            (16, 0),
        );
        assert_eq!(inactive, "New York 09:00:00 EDT (opens in 0h 30m)");
    }
}
