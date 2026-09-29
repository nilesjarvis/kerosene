use super::{build_income_snapshot, recent_hourly_payments};
use crate::account_analytics::model::{
    BorrowLendInterestEntry, BorrowLendReserveState, BorrowLendSideState, BorrowLendTokenState,
    BorrowLendUserState,
};

use std::collections::HashMap;

fn reserve(
    oracle_px: &str,
    supply_yearly_rate: &str,
    borrow_yearly_rate: &str,
) -> BorrowLendReserveState {
    BorrowLendReserveState {
        borrow_yearly_rate: borrow_yearly_rate.to_string(),
        supply_yearly_rate: supply_yearly_rate.to_string(),
        oracle_px: oracle_px.to_string(),
    }
}

fn side(value: &str) -> BorrowLendSideState {
    BorrowLendSideState {
        value: value.to_string(),
    }
}

fn token_state(supply: &str, borrow: &str) -> BorrowLendTokenState {
    BorrowLendTokenState {
        supply: side(supply),
        borrow: side(borrow),
    }
}

fn interest(time: u64, token: &str, supply: &str, borrow: &str) -> BorrowLendInterestEntry {
    BorrowLendInterestEntry {
        time,
        token: token.to_string(),
        borrow: borrow.to_string(),
        supply: supply.to_string(),
        n_samples: None,
    }
}

#[test]
fn income_snapshot_skips_invalid_numeric_rows_and_reports_counts() {
    let user_state = BorrowLendUserState {
        token_to_state: vec![(0, token_state("10", "4")), (1, token_state("2", "0"))],
        health: "healthy".to_string(),
        health_factor: Some("10".to_string()),
    };
    let reserve_by_token = HashMap::from([
        (0, reserve("2", "0.10", "0.20")),
        (1, reserve("bad", "0.10", "0.20")),
    ]);
    let token_name_by_id = HashMap::from([(0, "USDC".to_string()), (1, "BAD".to_string())]);
    let interest_entries = vec![
        interest(1_000, "0", "5", "1"),
        interest(2_000, "0", "bad", "2"),
        interest(3_000, "0", "NaN", "0"),
    ];

    let snapshot = build_income_snapshot(
        user_state,
        &interest_entries,
        &reserve_by_token,
        &token_name_by_id,
    );

    assert_eq!(snapshot.token_rows.len(), 1);
    assert_eq!(snapshot.current_supply_usd, 20.0);
    assert_eq!(snapshot.current_borrow_usd, 8.0);
    assert!((snapshot.net_yearly_projection - 0.4).abs() < 1e-12);
    assert_eq!(snapshot.earned_total, 4.0);
    assert_eq!(snapshot.recent_hourly_payments.len(), 1);
    assert_eq!(snapshot.invalid_token_rows, 1);
    assert_eq!(snapshot.invalid_interest_rows, 2);
}

#[test]
fn income_snapshot_hourly_rows_join_supply_rate_by_symbol_or_index() {
    let user_state = BorrowLendUserState {
        token_to_state: vec![(0, token_state("10", "4"))],
        health: "healthy".to_string(),
        health_factor: None,
    };
    let reserve_by_token = HashMap::from([(0, reserve("2", "0.10", "0.20"))]);
    let token_name_by_id = HashMap::from([(0, "USDC".to_string())]);
    let interest_entries = vec![
        interest(1_000, "0", "5", "1"),
        interest(2_000, "USDC", "2", "0"),
        interest(3_000, "UNKNOWN", "3", "0"),
    ];

    let snapshot = build_income_snapshot(
        user_state,
        &interest_entries,
        &reserve_by_token,
        &token_name_by_id,
    );

    let by_time: HashMap<u64, f64> = snapshot
        .recent_hourly_payments
        .iter()
        .map(|payment| (payment.time, payment.supply_rate))
        .collect();
    assert_eq!(by_time.get(&1_000), Some(&0.1));
    assert_eq!(by_time.get(&2_000), Some(&0.1));
    assert_eq!(by_time.get(&3_000), Some(&0.0));
}

#[test]
fn income_snapshot_counts_each_invalid_token_once_and_ignores_missing_reserves() {
    for (px, supply_rate, borrow_rate, supply, borrow) in [
        ("bad", "0.1", "0.2", "10", "4"),
        ("2", "NaN", "0.2", "10", "4"),
        ("2", "0.1", "inf", "10", "4"),
        ("2", "0.1", "0.2", "bad", "4"),
        ("2", "0.1", "0.2", "10", "bad"),
        ("2", "0.1", "0.2", "1e308", "4"),
        ("2", "0.1", "0.2", "10", "1e308"),
        ("1", "2", "0.2", "1e308", "4"),
        ("bad", "bad", "bad", "bad", "bad"),
    ] {
        let snapshot = build_income_snapshot(
            BorrowLendUserState {
                token_to_state: vec![
                    (0, token_state(supply, borrow)),
                    (1, token_state("bad", "bad")),
                ],
                health: "healthy".to_string(),
                health_factor: Some("10".to_string()),
            },
            &[],
            &HashMap::from([(0, reserve(px, supply_rate, borrow_rate))]),
            &HashMap::new(),
        );

        assert_eq!(snapshot.invalid_token_rows, 1);
        assert!(snapshot.token_rows.is_empty());
        assert_eq!(snapshot.current_supply_usd, 0.0);
        assert_eq!(snapshot.current_borrow_usd, 0.0);
        assert_eq!(snapshot.net_yearly_projection, 0.0);
        assert_eq!(snapshot.health, "healthy");
        assert_eq!(snapshot.health_factor.as_deref(), Some("10"));
    }
}

#[test]
fn income_snapshot_keeps_signed_values_zero_prices_and_stable_projection_ties() {
    let snapshot = build_income_snapshot(
        BorrowLendUserState {
            token_to_state: vec![
                (0, token_state("10", "2")),
                (1, token_state("-6", "0")),
                (2, token_state("4", "-1")),
            ],
            health: "healthy".to_string(),
            health_factor: None,
        },
        &[],
        &HashMap::from([
            (0, reserve("2", "0.25", "0.5")),
            (1, reserve("1", "0.5", "0")),
            (2, reserve("0", "0.1", "0.2")),
        ]),
        &HashMap::from([(0, "USDC".to_string())]),
    );

    let rows: Vec<_> = snapshot
        .token_rows
        .iter()
        .map(|row| {
            (
                row.token,
                row.token_label.as_str(),
                row.supply_usd,
                row.borrow_usd,
                row.supply_rate,
                row.net_yearly_usd,
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (0, "USDC", 20.0, 4.0, 0.25, 3.0),
            (1, "#1", -6.0, 0.0, 0.5, -3.0),
            (2, "#2", 0.0, 0.0, 0.1, 0.0),
        ]
    );
    assert_eq!(snapshot.invalid_token_rows, 0);
    assert_eq!(snapshot.current_supply_usd, 14.0);
    assert_eq!(snapshot.current_borrow_usd, 4.0);
    assert_eq!(snapshot.net_yearly_projection, 0.0);
}

#[test]
fn recent_payments_limit_valid_hourly_rows_and_preserve_timestamp_ties() {
    let mut entries = vec![
        interest(90, "first tie", "3", "1"),
        interest(1, "old", "3", "1"),
        interest(90, "second tie", "3", "1"),
        interest(90, "third tie", "3", "1"),
    ];
    entries.extend((100..110).map(|time| interest(time, "USDC", "3", "1")));
    entries.extend([
        interest(1_000, "bad supply", "NaN", "1"),
        interest(999, "bad borrow", "3", "bad"),
        interest(998, "overflow", "1e308", "-1e308"),
        BorrowLendInterestEntry {
            n_samples: Some(0),
            ..interest(997, "aggregate", "3", "1")
        },
        BorrowLendInterestEntry {
            n_samples: Some(24),
            ..interest(996, "aggregate", "3", "1")
        },
    ]);

    let payments = recent_hourly_payments(
        &entries,
        &HashMap::from([(0, reserve("1", "0.1", "0.2"))]),
        &HashMap::from([(0, "USDC".to_string())]),
    );

    assert_eq!(payments.len(), 12);
    let times: Vec<_> = payments.iter().map(|payment| payment.time).collect();
    assert_eq!(
        times,
        vec![109, 108, 107, 106, 105, 104, 103, 102, 101, 100, 90, 90]
    );
    assert_eq!(payments[10].token_label, "first tie");
    assert_eq!(payments[11].token_label, "second tie");
    for (index, payment) in payments.iter().enumerate() {
        assert_eq!(payment.supply, 3.0);
        assert_eq!(payment.borrow, 1.0);
        assert_eq!(payment.net, 2.0);
        assert_eq!(payment.supply_rate, if index < 10 { 0.1 } else { 0.0 });
    }
}

#[test]
fn recent_payments_keep_numeric_token_precedence_and_default_invalid_rates() {
    let entries = vec![
        interest(1, "0", "3", "1"),
        interest(1, "1", "3", "1"),
        interest(1, "2", "3", "1"),
        interest(1, "UNKNOWN", "3", "1"),
    ];
    let payments = recent_hourly_payments(
        &entries,
        &HashMap::from([
            (0, reserve("1", "0.1", "0.2")),
            (1, reserve("1", "NaN", "0.2")),
            (3, reserve("1", "0.3", "0.2")),
        ]),
        &HashMap::from([(0, "USDC".to_string()), (3, "2".to_string())]),
    );

    let labels_and_rates: Vec<_> = payments
        .iter()
        .map(|payment| (payment.token_label.as_str(), payment.supply_rate))
        .collect();
    assert_eq!(
        labels_and_rates,
        vec![("USDC", 0.1), ("1", 0.0), ("2", 0.0), ("UNKNOWN", 0.0)]
    );
}
