pub(in crate::chart) use crate::market_sessions::{
    MarketSession as SessionIndicatorKind, MarketSessionRange as SessionIndicatorRange,
    visible_session_ranges,
};

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
