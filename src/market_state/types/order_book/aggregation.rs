use crate::api::BookLevel;
use crate::helpers::aggregate_levels;

// ---------------------------------------------------------------------------
// Order Book Aggregation
// ---------------------------------------------------------------------------

/// Cached `(price, base_size, cumulative_total)` levels for the depth view,
/// keyed by book revision, tick size, and the cumulative total's unit.
#[derive(Debug, Default, Clone)]
pub struct AggregatedDepth {
    pub bids: Vec<(f64, f64, f64)>,
    pub asks: Vec<(f64, f64, f64)>,
    pub(in crate::market_state::types) book_revision: u64,
    pub(in crate::market_state::types) tick_bits: u64,
    pub(in crate::market_state::types) total_in_notional: bool,
}

pub(in crate::market_state::types) fn aggregate_with_cumulative(
    levels: &[BookLevel],
    tick: f64,
    is_bid: bool,
    total_in_notional: bool,
) -> Vec<(f64, f64, f64)> {
    let bucketed = aggregate_levels(levels, tick, is_bid);
    let mut cum = 0.0;
    bucketed
        .into_iter()
        .map(|(px, sz, notional)| {
            cum += if total_in_notional { notional } else { sz };
            (px, sz, cum)
        })
        .collect()
}
