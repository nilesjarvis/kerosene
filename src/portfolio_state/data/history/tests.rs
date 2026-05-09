use super::{apply_cutoff_with_baseline, compute_daily_pnl_rows_from_cumulative};

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
