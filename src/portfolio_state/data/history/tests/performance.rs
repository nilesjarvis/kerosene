use super::*;

#[test]
fn percent_performance_series_uses_pnl_delta_over_prior_account_value() {
    let pnl_points = vec![(1_000, 0.0), (2_000, 10.0)];
    let account_values = vec![(1_000, 1_000.0)];

    assert_point_series_near(
        &compute_percent_performance_series(&pnl_points, &account_values),
        &[(1_000, 0.0), (2_000, 1.0)],
    );
}

#[test]
fn percent_performance_series_uses_later_denominators_without_retroactive_skew() {
    let pnl_points = vec![(1_000, 0.0), (2_000, 10.0), (3_000, 30.0)];
    let account_values = vec![(1_000, 1_000.0), (2_000, 2_000.0)];

    assert_point_series_near(
        &compute_percent_performance_series(&pnl_points, &account_values),
        &[(1_000, 0.0), (2_000, 1.0), (3_000, 2.01)],
    );
}

#[test]
fn percent_performance_series_returns_none_without_reliable_account_values() {
    let pnl_points = vec![(1_000, 0.0), (2_000, 10.0)];

    assert!(compute_percent_performance_series(&pnl_points, &[]).is_empty());
    assert!(compute_percent_performance_series(&pnl_points, &[(1_000, 0.0)]).is_empty());
    assert!(compute_percent_performance_series(&pnl_points, &[(1_000, -1.0)]).is_empty());
    assert!(compute_percent_performance_series(&pnl_points, &[(1_000, 3.0)]).is_empty());
    assert!(compute_percent_performance_series(&pnl_points, &[(1_000, f64::NAN)]).is_empty());
}

#[test]
fn percent_performance_series_calculates_from_cutoff_baseline() {
    let pnl_points =
        apply_cutoff_with_baseline(&[(1_000, 0.0), (2_000, 5.0), (3_000, 15.0)], 2_500);
    let account_values = apply_cutoff_with_baseline(
        &[(1_000, 1_000.0), (2_000, 1_000.0), (3_000, 1_000.0)],
        2_500,
    );

    assert_point_series_near(
        &compute_percent_performance_series(&pnl_points, &account_values),
        &[(2_500, 0.0), (3_000, 1.0)],
    );
}
