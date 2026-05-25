use crate::api::BookLevel;
use crate::helpers::aggregate_levels;

// ---------------------------------------------------------------------------
// Order Book Aggregation
// ---------------------------------------------------------------------------

/// Cached `(price, size, cum_size)` levels for the depth view, keyed by the
/// book revision and tick size they were computed against.
#[derive(Debug, Default, Clone)]
pub struct AggregatedDepth {
    pub bids: Vec<(f64, f64, f64)>,
    pub asks: Vec<(f64, f64, f64)>,
    pub(in crate::market_state::types) book_revision: u64,
    pub(in crate::market_state::types) tick_bits: u64,
}

pub(in crate::market_state::types) fn aggregate_with_cumulative(
    levels: &[BookLevel],
    tick: f64,
    is_bid: bool,
) -> Vec<(f64, f64, f64)> {
    let bucketed = aggregate_levels(levels, tick, is_bid);
    let mut cum = 0.0;
    bucketed
        .into_iter()
        .map(|(px, sz)| {
            cum += sz;
            (px, sz, cum)
        })
        .collect()
}
