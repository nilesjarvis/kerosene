use crate::helpers::positive_finite_value;

use std::collections::HashMap;

pub(crate) const LIVE_MID_MAX_AGE_MS: u64 = 15_000;

pub(super) fn valid_mid_price(price: f64) -> bool {
    positive_finite_value(price).is_some()
}

fn live_mid_is_fresh(updated_at_ms: u64, now_ms: u64) -> bool {
    now_ms
        .checked_sub(updated_at_ms)
        .is_some_and(|age_ms| age_ms <= LIVE_MID_MAX_AGE_MS)
}

pub(super) fn resolve_live_mid_from_candidates(
    candidates: &[String],
    all_mids: &HashMap<String, f64>,
    all_mids_updated_at_ms: &HashMap<String, u64>,
    now_ms: u64,
) -> Option<f64> {
    for candidate in candidates {
        if let (Some(price), Some(updated_at_ms)) = (
            all_mids.get(candidate).copied(),
            all_mids_updated_at_ms.get(candidate).copied(),
        ) && valid_mid_price(price)
            && live_mid_is_fresh(updated_at_ms, now_ms)
        {
            return Some(price);
        }
    }
    None
}
