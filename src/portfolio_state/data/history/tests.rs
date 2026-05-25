use super::{
    apply_cutoff_with_baseline, compute_daily_percent_rows_from_cumulative,
    compute_daily_pnl_rows_from_cumulative, compute_percent_performance_series,
};

mod cutoff;
mod daily_percent;
mod daily_pnl;
mod performance;

fn assert_point_series_near(actual: &[(u64, f64)], expected: &[(u64, f64)]) {
    assert_eq!(actual.len(), expected.len());
    for ((actual_ts, actual_value), (expected_ts, expected_value)) in actual.iter().zip(expected) {
        assert_eq!(actual_ts, expected_ts);
        assert!(
            (actual_value - expected_value).abs() < 1e-9,
            "expected {expected_value}, got {actual_value}"
        );
    }
}
