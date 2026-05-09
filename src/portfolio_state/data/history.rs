use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Portfolio History Helpers
// ---------------------------------------------------------------------------

pub(super) fn apply_cutoff_with_baseline(points: &[(u64, f64)], cutoff: u64) -> Vec<(u64, f64)> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut baseline: Option<f64> = None;
    let mut filtered: Vec<(u64, f64)> = Vec::new();
    for (ts, value) in points {
        if *ts <= cutoff {
            baseline = Some(*value);
        }
        if *ts >= cutoff {
            filtered.push((*ts, *value));
        }
    }

    if let Some(base) = baseline {
        if filtered.is_empty() {
            return vec![(cutoff, base)];
        }
        if filtered.first().is_some_and(|(ts, _)| *ts > cutoff) {
            filtered.insert(0, (cutoff, base));
        }
    }

    filtered
}

pub(super) fn compute_daily_pnl_rows_from_cumulative(
    points: &[(u64, f64)],
    max_days: usize,
) -> Vec<(String, f64)> {
    let mut day_bounds: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    for (ts, value) in points {
        let Ok(ts_i64) = i64::try_from(*ts) else {
            continue;
        };
        let Some(dt) = DateTime::<Utc>::from_timestamp_millis(ts_i64) else {
            continue;
        };
        let key = dt.format("%Y-%m-%d").to_string();
        day_bounds
            .entry(key)
            .and_modify(|(_, last)| *last = *value)
            .or_insert((*value, *value));
    }

    let mut rows = Vec::new();
    let mut prev_day_last: Option<f64> = None;
    for (day, (first, last)) in day_bounds {
        let pnl = if let Some(prev) = prev_day_last {
            last - prev
        } else {
            last - first
        };
        prev_day_last = Some(last);
        rows.push((day, pnl));
    }

    rows.reverse();
    rows.truncate(max_days);
    rows
}
