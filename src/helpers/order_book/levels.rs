use crate::api::BookLevel;

use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// Round a normalization maximum up to the next 1-2-5 "nice" step.
///
/// Depth bars and heat intensities divide by the window maximum; using the
/// raw per-frame maximum makes every bar rescale on every book update. The
/// quantized step only changes when the raw maximum crosses a 1-2-5
/// boundary, so the scale stays put while the market ticks.
pub fn nice_step_ceil(value: f64) -> f64 {
    if !value.is_finite() || value <= 1.0 {
        return 1.0;
    }
    let exponent = value.log10().floor();
    let base = 10f64.powf(exponent);
    let mantissa = value / base;
    let stepped = if mantissa <= 1.0 {
        1.0
    } else if mantissa <= 2.0 {
        2.0
    } else if mantissa <= 5.0 {
        5.0
    } else {
        10.0
    };
    stepped * base
}

/// Tolerance (in tick units) for snapping a price/tick ratio onto the grid.
///
/// Binary float division of a grid-aligned price by its tick often lands an
/// epsilon away from the exact integer key (63.239 / 0.001 =
/// 63238.99999999999), so unguarded `floor`/`ceil` would shift such levels a
/// full tick down/up. Genuinely off-grid prices sit a large fraction of a
/// tick away from the grid, far outside this tolerance.
const GRID_SNAP_TOLERANCE: f64 = 1e-6;

/// Integer bucket key for a price at the given tick: grid-aligned prices snap
/// to their exact key; off-grid prices round down (bids) or up (asks).
fn bucket_key(px: f64, tick: f64, is_bid: bool) -> i64 {
    let scaled = px / tick;
    let nearest = scaled.round();
    let key = if (scaled - nearest).abs() <= GRID_SNAP_TOLERANCE {
        nearest
    } else if is_bid {
        scaled.floor()
    } else {
        scaled.ceil()
    };
    key as i64
}

/// Aggregate raw book levels into buckets at the given tick size.
pub fn aggregate_levels(levels: &[BookLevel], tick: f64, is_bid: bool) -> Vec<(f64, f64)> {
    let mut buckets: BTreeMap<i64, f64> = BTreeMap::new();

    for lvl in levels {
        *buckets
            .entry(bucket_key(lvl.px, tick, is_bid))
            .or_insert(0.0) += lvl.sz;
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
