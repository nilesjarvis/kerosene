use serde::{Deserialize, Serialize};

use super::HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS;

// ---------------------------------------------------------------------------
// HyperDash Data Models
// ---------------------------------------------------------------------------

/// A single liquidation entry from the HyperDash API.
/// `amount` is positive for longs (liquidated on drop) and negative for shorts
/// (liquidated on rise).
#[derive(Debug, Clone, Deserialize)]
pub struct LiquidationEntry {
    pub amount: f64,
    pub price: f64,
}

/// Aggregated liquidation data for a coin within a price range.
#[derive(Debug, Clone)]
pub struct LiquidationLevel {
    pub coin: String,
    pub min: f64,
    pub max: f64,
    pub liquidations: Vec<LiquidationEntry>,
}

/// A single bucket in the aggregated liquidation heatmap.
/// The API returns `amount` denominated in coins (not USD), so we track both
/// the raw coin size and the USD notional (amount * liquidation_price).
#[derive(Debug, Clone)]
pub struct LiquidationBucket {
    pub price_center: f64,
    /// Coin-denominated long size in this bucket.
    pub long_coins: f64,
    /// Coin-denominated short size in this bucket.
    pub short_coins: f64,
    /// USD notional of long liquidations (sum of |amount| * price).
    pub long_usd: f64,
    /// USD notional of short liquidations (sum of |amount| * price).
    pub short_usd: f64,
}

/// A single cell in the liquidation heatmap grid: one (time, price-band) pair.
#[derive(Debug, Clone)]
pub struct HeatmapRect {
    /// Timestamp for this cell (epoch milliseconds).
    pub timestamp_ms: u64,
    /// Width of this heatmap bucket on the time axis, in milliseconds.
    pub duration_ms: u64,
    /// Lower bound of the price band.
    pub price_lo: f64,
    /// Upper bound of the price band.
    pub price_hi: f64,
    /// Signed coin amount: positive = longs, negative = shorts.
    pub amount_coins: f64,
    /// USD notional: |amount_coins| * band midpoint price.
    pub amount_usd: f64,
}

/// Full historical liquidation heatmap data from HyperDash.
#[derive(Debug, Clone)]
pub struct LiquidationHeatmap {
    /// Flattened grid of renderable heatmap cells.
    pub rects: Vec<HeatmapRect>,
    /// Maximum absolute USD value across all cells (for color normalization).
    pub max_abs_usd: f64,
}

/// Parameters used for the last heatmap fetch, so we can detect when the
/// visible range has changed enough to warrant a re-fetch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapFetchParams {
    pub coin: String,
    pub min_price: f64,
    pub max_price: f64,
    pub start_time: u64,
    pub end_time: u64,
}

impl HeatmapFetchParams {
    pub fn cache_key(&self) -> String {
        format!(
            "{}:{:.8}:{:.8}:{}:{}",
            self.coin, self.min_price, self.max_price, self.start_time, self.end_time
        )
    }

    /// Check whether the current visible range has diverged enough from the
    /// last fetch parameters to justify a re-fetch.
    pub fn needs_refetch(
        &self,
        coin: &str,
        min_price: f64,
        max_price: f64,
        start: u64,
        end: u64,
    ) -> bool {
        if self.coin != coin {
            return true;
        }
        if end > self.end_time
            && end.saturating_sub(self.end_time) >= HYPERDASH_HEATMAP_DEFAULT_BUCKET_SECS
        {
            return true;
        }
        let old_range = self.max_price - self.min_price;
        if old_range <= 0.0 {
            return true;
        }
        let price_shift =
            ((min_price - self.min_price).abs() + (max_price - self.max_price).abs()) / old_range;
        if price_shift > 0.3 {
            return true;
        }
        let old_dur = self.end_time.saturating_sub(self.start_time);
        if old_dur == 0 {
            return true;
        }
        let time_shift =
            (start.abs_diff(self.start_time) + end.abs_diff(self.end_time)) as f64 / old_dur as f64;
        time_shift > 0.3
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct GqlError {
    pub(super) message: String,
}
