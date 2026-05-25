use super::build_income_snapshot;
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
