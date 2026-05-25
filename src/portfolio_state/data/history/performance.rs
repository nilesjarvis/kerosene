use super::points::{
    latest_account_value_at_or_before, sorted_finite_points, sorted_reliable_account_values,
};
use crate::helpers::finite_value;

// ---------------------------------------------------------------------------
// Portfolio Performance Series
// ---------------------------------------------------------------------------

pub(in crate::portfolio_state::data) fn compute_percent_performance_series(
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
        let Some(period_return) = finite_value(pnl_delta / prior_account_value) else {
            continue;
        };

        if series.is_empty() {
            series.push((prior_ts, 0.0));
        }

        cumulative_return *= 1.0 + period_return;
        if let Some(performance_percent) = finite_value((cumulative_return - 1.0) * 100.0) {
            series.push((current_ts, performance_percent));
        }
    }

    series
}
