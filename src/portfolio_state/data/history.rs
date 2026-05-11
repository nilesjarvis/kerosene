use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Portfolio History Helpers
// ---------------------------------------------------------------------------

const MIN_RELIABLE_ACCOUNT_VALUE: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
struct DayBounds {
    first_ts: u64,
    first_value: f64,
    last_ts: u64,
    last_value: f64,
}

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

pub(super) fn compute_percent_performance_series(
    pnl_points: &[(u64, f64)],
    account_value_points: &[(u64, f64)],
) -> Vec<(u64, f64)> {
    let pnl_points = sorted_finite_points(pnl_points);
    if pnl_points.len() < 2 {
        return Vec::new();
    }

    let account_value_points = sorted_reliable_account_values(account_value_points);
    if account_value_points.is_empty() {
        return Vec::new();
    }

    let mut cumulative_return = 1.0;
    let mut series = Vec::new();

    for period in pnl_points.windows(2) {
        let (prior_ts, prior_pnl) = period[0];
        let (current_ts, current_pnl) = period[1];
        if current_ts <= prior_ts {
            continue;
        }

        let Some(prior_account_value) =
            latest_account_value_at_or_before(&account_value_points, prior_ts)
        else {
            continue;
        };

        let pnl_delta = current_pnl - prior_pnl;
        let period_return = pnl_delta / prior_account_value;
        if !period_return.is_finite() {
            continue;
        }

        if series.is_empty() {
            series.push((prior_ts, 0.0));
        }

        cumulative_return *= 1.0 + period_return;
        let performance_percent = (cumulative_return - 1.0) * 100.0;
        if performance_percent.is_finite() {
            series.push((current_ts, performance_percent));
        }
    }

    series
}

pub(super) fn compute_daily_pnl_rows_from_cumulative(
    points: &[(u64, f64)],
    max_days: usize,
) -> Vec<(String, f64)> {
    let day_bounds = daily_bounds_from_cumulative(points);

    let mut rows = Vec::new();
    let mut prev_day_last: Option<f64> = None;
    for (day, bounds) in day_bounds {
        let pnl = if let Some(prev) = prev_day_last {
            bounds.last_value - prev
        } else {
            bounds.last_value - bounds.first_value
        };
        prev_day_last = Some(bounds.last_value);
        rows.push((day, pnl));
    }

    rows.reverse();
    rows.truncate(max_days);
    rows
}

pub(super) fn compute_daily_percent_rows_from_cumulative(
    pnl_points: &[(u64, f64)],
    account_value_points: &[(u64, f64)],
    max_days: usize,
) -> Vec<(String, f64)> {
    let account_value_points = sorted_reliable_account_values(account_value_points);
    if account_value_points.is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    let mut prev_day_last: Option<(u64, f64)> = None;
    for (day, bounds) in daily_bounds_from_cumulative(pnl_points) {
        let (period_start_ts, period_start_value) =
            prev_day_last.unwrap_or((bounds.first_ts, bounds.first_value));
        let pnl_delta = bounds.last_value - period_start_value;
        prev_day_last = Some((bounds.last_ts, bounds.last_value));

        let Some(prior_account_value) =
            latest_account_value_at_or_before(&account_value_points, period_start_ts)
        else {
            continue;
        };

        let daily_percent = pnl_delta / prior_account_value * 100.0;
        if daily_percent.is_finite() {
            rows.push((day, daily_percent));
        }
    }

    rows.reverse();
    rows.truncate(max_days);
    rows
}

fn daily_bounds_from_cumulative(points: &[(u64, f64)]) -> BTreeMap<String, DayBounds> {
    let mut day_bounds: BTreeMap<String, DayBounds> = BTreeMap::new();

    for (ts, value) in sorted_finite_points(points) {
        let Ok(ts_i64) = i64::try_from(ts) else {
            continue;
        };
        let Some(dt) = DateTime::<Utc>::from_timestamp_millis(ts_i64) else {
            continue;
        };
        let key = dt.format("%Y-%m-%d").to_string();
        day_bounds
            .entry(key)
            .and_modify(|bounds| {
                bounds.last_ts = ts;
                bounds.last_value = value;
            })
            .or_insert(DayBounds {
                first_ts: ts,
                first_value: value,
                last_ts: ts,
                last_value: value,
            });
    }

    day_bounds
}

fn latest_account_value_at_or_before(points: &[(u64, f64)], timestamp_ms: u64) -> Option<f64> {
    points
        .iter()
        .take_while(|(ts, _)| *ts <= timestamp_ms)
        .last()
        .map(|(_, value)| *value)
}

fn sorted_finite_points(points: &[(u64, f64)]) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .copied()
        .filter(|(_, value)| value.is_finite())
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(ts, _)| *ts);
    sorted
}

fn sorted_reliable_account_values(points: &[(u64, f64)]) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .copied()
        .filter(|(_, value)| is_reliable_account_value(*value))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(ts, _)| *ts);
    sorted
}

fn is_reliable_account_value(value: f64) -> bool {
    value.is_finite() && value > MIN_RELIABLE_ACCOUNT_VALUE
}
