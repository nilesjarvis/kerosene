use super::*;

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
