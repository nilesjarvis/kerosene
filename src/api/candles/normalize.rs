use super::model::Candle;

pub fn is_valid_candle(candle: &Candle) -> bool {
    candle.open_time > 0
        && candle.close_time >= candle.open_time
        && candle.open.is_finite()
        && candle.high.is_finite()
        && candle.low.is_finite()
        && candle.close.is_finite()
        && candle.volume.is_finite()
        && candle.volume >= 0.0
        && candle.low <= candle.high
        && candle.low <= candle.open
        && candle.low <= candle.close
        && candle.high >= candle.open
        && candle.high >= candle.close
}

pub fn normalize_candles(mut candles: Vec<Candle>) -> Vec<Candle> {
    candles.retain(is_valid_candle);
    candles.sort_by_key(|candle| candle.open_time);

    candles.dedup_by(|next, previous| {
        if next.open_time != previous.open_time {
            return false;
        }
        // dedup_by removes its first argument. Retain the last valid input
        // for each timestamp by swapping it into the surviving slot.
        std::mem::swap(next, previous);
        true
    });
    candles
}

// ---------------------------------------------------------------------------
// Contiguity / gap detection
//
// A continuous-market series should be exactly interval-spaced. Missing data
// can come from an exchange/stream outage, a sleep/wake reconnect, or a stale
// cache stitch (an old block joined to a fresh tail). Such a series renders as
// a price jump because the chart positions candles by index, not wall-clock
// time, so continuous-market cache paths use the exact helpers below. Sparse
// markets use the separately named tolerant helpers so one or two legitimately
// empty trade buckets do not discard their warm-start tail.
// ---------------------------------------------------------------------------

/// Adjacent candles separated by more than this many intervals are treated as a
/// genuine discontinuity rather than ordinary sparse data.
pub const MAX_CONTIGUOUS_GAP_INTERVALS: u64 = 3;

/// Largest open-time delta between adjacent candles still considered contiguous
/// for `interval_ms`. `None` when the interval is unknown (0) — callers then
/// make no contiguity decision and treat the series as contiguous.
fn max_contiguous_gap_ms(interval_ms: u64) -> Option<u64> {
    (interval_ms != 0).then(|| interval_ms.saturating_mul(MAX_CONTIGUOUS_GAP_INTERVALS))
}

/// Whether `new_open_time` skips at least one expected bucket.
pub fn open_time_starts_after_gap(
    last_open_time: u64,
    new_open_time: u64,
    interval_ms: u64,
) -> bool {
    interval_ms != 0 && new_open_time > last_open_time.saturating_add(interval_ms)
}

/// Whether an open-time-sorted series omits at least one expected bucket.
pub fn candles_have_missing_intervals(candles: &[Candle], interval_ms: u64) -> bool {
    interval_ms != 0
        && candles
            .windows(2)
            .any(|pair| pair[1].open_time > pair[0].open_time.saturating_add(interval_ms))
}

/// Whether adjacent buckets fail to advance by exactly one interval.
///
/// Normalization has already sorted and deduplicated the series, so a shorter
/// delta is malformed/overlapping data and a longer delta is a missing bucket.
pub fn candles_have_interval_discontinuity(candles: &[Candle], interval_ms: u64) -> bool {
    interval_ms != 0
        && candles
            .windows(2)
            .any(|pair| pair[1].open_time != pair[0].open_time.saturating_add(interval_ms))
}

/// Whether any adjacent pair in an open-time-sorted series is separated by a gap
/// large enough to be a real discontinuity for `interval_ms`.
pub fn candles_have_interior_gap(candles: &[Candle], interval_ms: u64) -> bool {
    let Some(max_gap) = max_contiguous_gap_ms(interval_ms) else {
        return false;
    };
    candles
        .windows(2)
        .any(|pair| pair[1].open_time.saturating_sub(pair[0].open_time) > max_gap)
}

/// Index of the first candle in the trailing run of contiguous candles: the
/// suffix `&candles[start..]` has no interior gap for `interval_ms`. Returns 0
/// when the whole (open-time-sorted) series is contiguous or the interval is
/// unknown, so a healthy series is never trimmed.
pub fn trailing_contiguous_run_start(candles: &[Candle], interval_ms: u64) -> usize {
    let Some(max_gap) = max_contiguous_gap_ms(interval_ms) else {
        return 0;
    };
    candles
        .windows(2)
        .rposition(|pair| pair[1].open_time.saturating_sub(pair[0].open_time) > max_gap)
        .map_or(0, |index| index + 1)
}

/// Index of the first candle in the trailing exactly interval-spaced run.
pub fn trailing_exact_run_start(candles: &[Candle], interval_ms: u64) -> usize {
    if interval_ms == 0 {
        return 0;
    }
    candles
        .windows(2)
        .rposition(|pair| pair[1].open_time != pair[0].open_time.saturating_add(interval_ms))
        .map_or(0, |index| index + 1)
}

#[cfg(test)]
mod gap_tests {
    use super::*;
    use crate::api::Candle;

    fn series(open_times: &[u64]) -> Vec<Candle> {
        open_times
            .iter()
            .map(|&t| Candle::test_flat(t, 100.0))
            .collect()
    }

    #[test]
    fn contiguous_series_has_no_interior_gap_and_is_not_trimmed() {
        let candles = series(&[60_000, 120_000, 180_000, 240_000]);
        assert!(!candles_have_interior_gap(&candles, 60_000));
        assert_eq!(trailing_contiguous_run_start(&candles, 60_000), 0);
    }

    #[test]
    fn small_gaps_are_tolerated() {
        // One missing candle (delta == 2 intervals) is within tolerance.
        let candles = series(&[60_000, 180_000, 240_000]);
        assert!(!candles_have_interior_gap(&candles, 60_000));
        assert!(candles_have_missing_intervals(&candles, 60_000));
        assert!(candles_have_interval_discontinuity(&candles, 60_000));
        assert_eq!(trailing_contiguous_run_start(&candles, 60_000), 0);
        assert_eq!(trailing_exact_run_start(&candles, 60_000), 1);
    }

    #[test]
    fn exact_run_rejects_overlapping_or_misaligned_buckets() {
        let candles = series(&[60_000, 90_000, 150_000]);
        assert!(!candles_have_missing_intervals(&candles, 60_000));
        assert!(candles_have_interval_discontinuity(&candles, 60_000));
        assert_eq!(trailing_exact_run_start(&candles, 60_000), 1);
    }

    #[test]
    fn large_interior_gap_is_detected_and_trailing_run_starts_after_it() {
        // Old block, then a multi-interval hole, then a recent block.
        let candles = series(&[60_000, 120_000, 10_000_000, 10_060_000]);
        assert!(candles_have_interior_gap(&candles, 60_000));
        assert_eq!(trailing_contiguous_run_start(&candles, 60_000), 2);
    }

    #[test]
    fn trailing_run_uses_the_last_gap_when_several_exist() {
        let candles = series(&[1_000_000, 5_000_000, 5_060_000, 9_000_000, 9_060_000]);
        assert_eq!(trailing_contiguous_run_start(&candles, 60_000), 3);
    }

    #[test]
    fn open_time_gap_detection_matches_thresholds() {
        assert!(!open_time_starts_after_gap(60_000, 120_000, 60_000)); // next candle
        assert!(open_time_starts_after_gap(60_000, 180_000, 60_000)); // one skip
        assert!(open_time_starts_after_gap(60_000, 600_000, 60_000)); // real gap
    }

    #[test]
    fn unknown_interval_makes_no_contiguity_decision() {
        let candles = series(&[1, 10_000_000]);
        assert!(!candles_have_interior_gap(&candles, 0));
        assert!(!candles_have_missing_intervals(&candles, 0));
        assert!(!candles_have_interval_discontinuity(&candles, 0));
        assert_eq!(trailing_contiguous_run_start(&candles, 0), 0);
        assert_eq!(trailing_exact_run_start(&candles, 0), 0);
        assert!(!open_time_starts_after_gap(1, 10_000_000, 0));
    }

    #[test]
    fn trailing_runs_preserve_gap_boundaries_and_saturating_arithmetic() {
        let cases: &[(&[u64], u64, usize, usize)] = &[
            (&[], 1, 0, 0),
            (&[1], 1, 0, 0),
            (&[1, 1, 2], 1, 0, 1),
            (&[3, 2, 3], 1, 0, 1),
            (&[1, 4, 5], 1, 0, 1),
            (&[1, 5, 6], 1, 1, 1),
            (&[1, 2, 9], 1, 2, 2),
            (&[1, 9, 10, 20], 1, 3, 3),
            (&[u64::MAX - 1, u64::MAX, u64::MAX], 1, 0, 0),
            (&[1, u64::MAX], u64::MAX, 0, 0),
            (&[0, u64::MAX], u64::MAX / 3, 0, 1),
            (&[1, u64::MAX], 0, 0, 0),
        ];
        for &(times, interval, tolerant, exact) in cases {
            // Build only the timestamps these helpers inspect; the extreme values
            // deliberately include values outside valid candle payloads.
            let candles: Vec<_> = times
                .iter()
                .map(|&time| Candle {
                    open_time: time,
                    close_time: time,
                    ..Candle::test_flat(0, 1.0)
                })
                .collect();
            assert_eq!(
                trailing_contiguous_run_start(&candles, interval),
                tolerant,
                "{times:?}, interval {interval}"
            );
            assert_eq!(
                trailing_exact_run_start(&candles, interval),
                exact,
                "{times:?}, interval {interval}"
            );
        }
    }
}
