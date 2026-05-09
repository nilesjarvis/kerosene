use super::numbers::parse_f64_str;
use crate::account_analytics::model::BorrowLendInterestEntry;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Interest Deduplication
// ---------------------------------------------------------------------------

pub(in crate::account_analytics::income) struct IncomePerDayDedup {
    pub(in crate::account_analytics::income) values: HashMap<u64, f64>,
    pub(in crate::account_analytics::income) invalid_rows: usize,
}

#[cfg(test)]
pub(in crate::account_analytics::income) fn income_per_day_dedup(
    entries: &[BorrowLendInterestEntry],
) -> HashMap<u64, f64> {
    income_per_day_dedup_with_stats(entries).values
}

pub(in crate::account_analytics::income) fn income_per_day_dedup_with_stats(
    entries: &[BorrowLendInterestEntry],
) -> IncomePerDayDedup {
    const DAY_MS: u64 = 24 * 60 * 60 * 1000;

    #[derive(Default)]
    struct DayBucket {
        intraday_sum: f64,
        aggregate_sum: f64,
        has_intraday: bool,
    }

    let mut buckets: HashMap<u64, DayBucket> = HashMap::new();
    let mut invalid_rows = 0_usize;
    for entry in entries {
        let day_start = entry.time.saturating_sub(entry.time % DAY_MS);
        let Some(supply) = parse_f64_str(&entry.supply) else {
            invalid_rows += 1;
            continue;
        };
        let Some(borrow) = parse_f64_str(&entry.borrow) else {
            invalid_rows += 1;
            continue;
        };
        let net = supply - borrow;
        let bucket = buckets.entry(day_start).or_default();
        if entry.n_samples.is_some() {
            bucket.aggregate_sum += net;
        } else {
            bucket.intraday_sum += net;
            bucket.has_intraday = true;
        }
    }

    let values = buckets
        .into_iter()
        .map(|(day, bucket)| {
            let value = if bucket.has_intraday {
                bucket.intraday_sum
            } else {
                bucket.aggregate_sum
            };
            (day, value)
        })
        .collect();

    IncomePerDayDedup {
        values,
        invalid_rows,
    }
}
