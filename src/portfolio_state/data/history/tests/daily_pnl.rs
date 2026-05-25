use super::*;

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
fn daily_pnl_rows_skip_nonfinite_values() {
    let points = vec![
        (1_672_531_200_000, 100.0),
        (1_672_574_400_000, f64::NAN),
        (1_672_617_600_000, 120.0),
    ];

    assert_eq!(
        compute_daily_pnl_rows_from_cumulative(&points, 2),
        vec![
            ("2023-01-02".to_string(), 20.0),
            ("2023-01-01".to_string(), 0.0),
        ]
    );
}
