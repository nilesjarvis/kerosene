use chrono::{Duration, TimeZone, Utc};
use chrono_tz::Tz;

// ---------------------------------------------------------------------------
// Session Indicator Schedule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::chart) enum SessionIndicatorKind {
    NewYork,
    Overnight,
    Asia,
    London,
}

impl SessionIndicatorKind {
    pub(in crate::chart) fn label(self) -> &'static str {
        match self {
            Self::NewYork => "NY",
            Self::Overnight => "O/N",
            Self::Asia => "Asia",
            Self::London => "London",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::chart) struct SessionIndicatorRange {
    pub(in crate::chart) kind: SessionIndicatorKind,
    pub(in crate::chart) start_ms: u64,
    pub(in crate::chart) end_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SessionBoundary {
    time_ms: u64,
    next_kind: SessionIndicatorKind,
}

pub(in crate::chart) fn visible_session_ranges(
    start_ms: u64,
    end_ms: u64,
) -> Vec<SessionIndicatorRange> {
    if end_ms <= start_ms {
        return Vec::new();
    }

    let boundaries = session_boundaries(start_ms, end_ms);
    boundaries
        .windows(2)
        .filter_map(|pair| {
            let start = pair[0].time_ms;
            let end = pair[1].time_ms;
            (end > start_ms && start < end_ms && end > start).then_some(SessionIndicatorRange {
                kind: pair[0].next_kind,
                start_ms: start,
                end_ms: end,
            })
        })
        .collect()
}

fn session_boundaries(start_ms: u64, end_ms: u64) -> Vec<SessionBoundary> {
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
        push_boundary(
            &mut boundaries,
            chrono_tz::Asia::Tokyo,
            date,
            9,
            0,
            SessionIndicatorKind::Asia,
        );
        push_boundary(
            &mut boundaries,
            chrono_tz::Europe::London,
            date,
            8,
            0,
            SessionIndicatorKind::London,
        );
        push_boundary(
            &mut boundaries,
            chrono_tz::America::New_York,
            date,
            9,
            30,
            SessionIndicatorKind::NewYork,
        );
        push_boundary(
            &mut boundaries,
            chrono_tz::America::New_York,
            date,
            16,
            0,
            SessionIndicatorKind::Overnight,
        );
        date += Duration::days(1);
    }

    boundaries.sort_by_key(|boundary| boundary.time_ms);
    boundaries.dedup_by_key(|boundary| boundary.time_ms);
    boundaries
}

fn push_boundary(
    boundaries: &mut Vec<SessionBoundary>,
    tz: Tz,
    date: chrono::NaiveDate,
    hour: u32,
    minute: u32,
    next_kind: SessionIndicatorKind,
) {
    if let Some(time_ms) = local_time_ms(tz, date, hour, minute) {
        boundaries.push(SessionBoundary { time_ms, next_kind });
    }
}

fn local_time_ms(tz: Tz, date: chrono::NaiveDate, hour: u32, minute: u32) -> Option<u64> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    let local = tz
        .from_local_datetime(&naive)
        .earliest()
        .or_else(|| tz.from_local_datetime(&naive).latest())?;
    u64::try_from(local.with_timezone(&Utc).timestamp_millis()).ok()
}

fn utc_from_ms(time_ms: u64) -> Option<chrono::DateTime<Utc>> {
    let time_ms = i64::try_from(time_ms).ok()?;
    Utc.timestamp_millis_opt(time_ms).single()
}

#[cfg(test)]
mod tests {
    use super::{SessionIndicatorKind, visible_session_ranges};
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
                    SessionIndicatorKind::Asia,
                    ts(2026, 1, 14, 0, 0),
                    ts(2026, 1, 14, 8, 0),
                ),
                (
                    SessionIndicatorKind::London,
                    ts(2026, 1, 14, 8, 0),
                    ts(2026, 1, 14, 14, 30),
                ),
                (
                    SessionIndicatorKind::NewYork,
                    ts(2026, 1, 14, 14, 30),
                    ts(2026, 1, 14, 21, 0),
                ),
                (
                    SessionIndicatorKind::Overnight,
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
                    SessionIndicatorKind::Asia,
                    ts(2026, 7, 14, 0, 0),
                    ts(2026, 7, 14, 7, 0),
                ),
                (
                    SessionIndicatorKind::London,
                    ts(2026, 7, 14, 7, 0),
                    ts(2026, 7, 14, 13, 30),
                ),
                (
                    SessionIndicatorKind::NewYork,
                    ts(2026, 7, 14, 13, 30),
                    ts(2026, 7, 14, 20, 0),
                ),
                (
                    SessionIndicatorKind::Overnight,
                    ts(2026, 7, 14, 20, 0),
                    ts(2026, 7, 15, 0, 0),
                ),
            ]
        );
    }

    #[test]
    fn empty_or_reversed_windows_return_no_ranges() {
        assert!(visible_session_ranges(1_000, 1_000).is_empty());
        assert!(visible_session_ranges(2_000, 1_000).is_empty());
    }
}
