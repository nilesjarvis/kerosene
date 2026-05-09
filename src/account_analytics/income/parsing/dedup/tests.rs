use crate::account_analytics::model::BorrowLendInterestEntry;

use super::{income_per_day_dedup, income_per_day_dedup_with_stats};

fn interest_entry(
    time: u64,
    supply: &str,
    borrow: &str,
    n_samples: Option<u32>,
) -> BorrowLendInterestEntry {
    BorrowLendInterestEntry {
        time,
        token: "0".to_string(),
        borrow: borrow.to_string(),
        supply: supply.to_string(),
        n_samples,
    }
}

#[test]
fn daily_income_prefers_intraday_rows_over_aggregate_rows() {
    let day_ms = 24 * 60 * 60 * 1000;
    let deduped = income_per_day_dedup(&[
        interest_entry(day_ms + 10, "5", "1", Some(24)),
        interest_entry(day_ms + 20, "2", "0.5", None),
        interest_entry(day_ms + 30, "3", "1.0", None),
    ]);

    assert_eq!(deduped.get(&day_ms).copied(), Some(3.5));
}

#[test]
fn daily_income_skips_invalid_numeric_rows() {
    let day_ms = 24 * 60 * 60 * 1000;
    let deduped = income_per_day_dedup(&[
        interest_entry(day_ms + 10, "5", "1", None),
        interest_entry(day_ms + 20, "bad", "2", None),
        interest_entry(day_ms + 30, "NaN", "0", None),
    ]);

    assert_eq!(deduped.get(&day_ms).copied(), Some(4.0));
}

#[test]
fn daily_income_counts_invalid_numeric_rows() {
    let day_ms = 24 * 60 * 60 * 1000;
    let deduped = income_per_day_dedup_with_stats(&[
        interest_entry(day_ms + 10, "5", "1", None),
        interest_entry(day_ms + 20, "bad", "2", None),
        interest_entry(day_ms + 30, "NaN", "0", None),
    ]);

    assert_eq!(deduped.values.get(&day_ms).copied(), Some(4.0));
    assert_eq!(deduped.invalid_rows, 2);
}
