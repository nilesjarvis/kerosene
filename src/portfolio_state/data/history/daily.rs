use super::points::{
    latest_account_value_at_or_before, sorted_finite_points, sorted_reliable_account_values,
};
use crate::helpers::finite_value;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Daily Portfolio Rows
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct DayBounds {
    first_ts: u64,
    first_value: f64,
    last_ts: u64,
    last_value: f64,
}

pub(in crate::portfolio_state::data) fn compute_daily_pnl_rows_from_cumulative(
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

pub(in crate::portfolio_state::data) fn compute_daily_percent_rows_from_cumulative(
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
        if let Some(daily_percent) = finite_value(daily_percent) {
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
