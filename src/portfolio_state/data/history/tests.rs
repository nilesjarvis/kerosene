use super::{
    apply_cutoff_with_baseline, compute_daily_percent_rows_from_cumulative,
    compute_daily_pnl_rows_from_cumulative, compute_percent_performance_series,
};

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

#[test]
fn cutoff_with_baseline_inserts_prior_value_at_cutoff() {
    let points = vec![(1_000, 10.0), (2_000, 20.0), (3_000, 30.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 2_500),
        vec![(2_500, 20.0), (3_000, 30.0)]
    );
}

#[test]
fn cutoff_with_baseline_returns_cutoff_value_when_all_points_are_before_cutoff() {
    let points = vec![(1_000, 10.0), (2_000, 20.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 3_000),
        vec![(3_000, 20.0)]
    );
}

#[test]
fn cutoff_with_baseline_keeps_existing_cutoff_point_without_duplicate() {
    let points = vec![(1_000, 10.0), (2_000, 20.0), (3_000, 30.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 2_000),
        vec![(2_000, 20.0), (3_000, 30.0)]
    );
}

#[test]
fn cutoff_without_baseline_returns_future_points_only() {
    let points = vec![(2_000, 20.0), (3_000, 30.0)];

    assert_eq!(
        apply_cutoff_with_baseline(&points, 1_000),
        vec![(2_000, 20.0), (3_000, 30.0)]
    );
}

#[test]
fn daily_pnl_rows_use_previous_day_close_and_return_newest_first() {
    let points = vec![
        (1_672_531_200_000, 100.0), // 2023-01-01 00:00 UTC
        (1_672_617_600_000, 110.0), // 2023-01-02 00:00 UTC
        (1_672_660_800_000, 125.0), // 2023-01-02 12:00 UTC
        (1_672_704_000_000, 130.0), // 2023-01-03 00:00 UTC
    ];

    assert_eq!(
        compute_daily_pnl_rows_from_cumulative(&points, 3),
        vec![
            ("2023-01-03".to_string(), 5.0),
            ("2023-01-02".to_string(), 25.0),
            ("2023-01-01".to_string(), 0.0),
        ]
    );
}

#[test]
fn daily_pnl_rows_skip_invalid_timestamps_and_truncate() {
    let points = vec![
        (u64::MAX, 999.0),
        (1_672_531_200_000, 100.0),
        (1_672_617_600_000, 110.0),
    ];

    assert_eq!(
        compute_daily_pnl_rows_from_cumulative(&points, 1),
        vec![("2023-01-02".to_string(), 10.0)]
    );
}

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

#[test]
fn daily_percent_rows_use_previous_day_close_and_prior_account_value() {
    let pnl_points = vec![
        (1_672_531_200_000, 100.0), // 2023-01-01 00:00 UTC
        (1_672_574_400_000, 110.0), // 2023-01-01 12:00 UTC
        (1_672_617_600_000, 130.0), // 2023-01-02 00:00 UTC
        (1_672_704_000_000, 160.0), // 2023-01-03 00:00 UTC
    ];
    let account_values = vec![(1_672_531_200_000, 1_000.0), (1_672_617_600_000, 2_000.0)];

    assert_eq!(
        compute_daily_percent_rows_from_cumulative(&pnl_points, &account_values, 3),
        vec![
            ("2023-01-03".to_string(), 1.5),
            ("2023-01-02".to_string(), 2.0),
            ("2023-01-01".to_string(), 1.0),
        ]
    );
}

#[test]
fn daily_percent_rows_skip_invalid_days_and_truncate() {
    let day_ms = 86_400_000;
    let start = 1_672_531_200_000;
    let pnl_points = (0..10)
        .map(|day| (start + day * day_ms, day as f64 * 10.0))
        .collect::<Vec<_>>();
    let account_values = (2..10)
        .map(|day| (start + day * day_ms, 1_000.0))
        .collect::<Vec<_>>();

    let rows = compute_daily_percent_rows_from_cumulative(&pnl_points, &account_values, 7);

    assert_eq!(rows.len(), 7);
    assert_eq!(rows.first(), Some(&("2023-01-10".to_string(), 1.0)));
    assert_eq!(rows.last(), Some(&("2023-01-04".to_string(), 1.0)));
}
