use crate::api::BookLevel;

use std::collections::BTreeMap;

/// Aggregate raw book levels into buckets at the given tick size.
pub fn aggregate_levels(levels: &[BookLevel], tick: f64, is_bid: bool) -> Vec<(f64, f64)> {
    let mut buckets: BTreeMap<i64, f64> = BTreeMap::new();

    for lvl in levels {
        let bucket_key = if is_bid {
            (lvl.px / tick).floor() as i64
        } else {
            (lvl.px / tick).ceil() as i64
        };
        *buckets.entry(bucket_key).or_insert(0.0) += lvl.sz;
    }

    let mut result: Vec<(f64, f64)> = buckets
        .into_iter()
        .map(|(k, sz)| (k as f64 * tick, sz))
        .collect();

    if is_bid {
        result.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }

    result
}
