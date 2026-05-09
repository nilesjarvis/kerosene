use super::super::models::{LiquidationBucket, LiquidationEntry};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Liquidation Bucketing
// ---------------------------------------------------------------------------

/// Aggregate raw liquidation entries into price buckets for heatmap rendering.
/// Returns buckets sorted by price (ascending), each covering `bucket_width`
/// price units. `num_buckets` controls the resolution.
///
/// The API returns `amount` in **coins** (not USD). Each entry's USD notional
/// is `|amount| * price`, which is what we use for bar sizing and display.
pub fn bucket_liquidations(
    liquidations: &[LiquidationEntry],
    price_lo: f64,
    price_hi: f64,
    num_buckets: usize,
) -> Vec<LiquidationBucket> {
    if num_buckets == 0 || price_hi <= price_lo || liquidations.is_empty() {
        return Vec::new();
    }

    let range = price_hi - price_lo;
    let bucket_width = range / num_buckets as f64;
    let mut buckets: Vec<LiquidationBucket> = (0..num_buckets)
        .map(|i| LiquidationBucket {
            price_center: price_lo + (i as f64 + 0.5) * bucket_width,
            long_coins: 0.0,
            short_coins: 0.0,
            long_usd: 0.0,
            short_usd: 0.0,
        })
        .collect();

    for entry in liquidations {
        if entry.price < price_lo || entry.price > price_hi {
            continue;
        }
        let idx = ((entry.price - price_lo) / bucket_width) as usize;
        let idx = idx.min(num_buckets - 1);
        let usd_notional = entry.amount.abs() * entry.price;
        if entry.amount > 0.0 {
            buckets[idx].long_coins += entry.amount;
            buckets[idx].long_usd += usd_notional;
        } else {
            buckets[idx].short_coins += entry.amount.abs();
            buckets[idx].short_usd += usd_notional;
        }
    }

    buckets
}
