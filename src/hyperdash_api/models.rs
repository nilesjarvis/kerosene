use std::fmt;

use crate::helpers::redact_sensitive_response_text;
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

/// A single wallet-level perp position for the Positioning Information widget.
#[derive(Clone, Deserialize, PartialEq)]
pub struct TickerPositionEntry {
    pub address: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub label: Option<String>,
    pub tag: Option<String>,
    pub verified: Option<bool>,
    #[serde(rename = "copyScore")]
    pub copy_score: Option<f64>,
    pub size: f64,
    #[serde(rename = "notionalSize")]
    pub notional_size: f64,
    #[serde(rename = "entryPrice")]
    pub entry_price: f64,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: Option<f64>,
    #[serde(rename = "unrealizedPnl")]
    pub unrealized_pnl: f64,
    #[serde(rename = "fundingPnl")]
    pub funding_pnl: f64,
    #[serde(rename = "accountValue")]
    pub account_value: f64,
}

impl fmt::Debug for TickerPositionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TickerPositionEntry")
            .field("address", &"<redacted>")
            .field(
                "display_name",
                &redacted_optional_wallet_debug_value(&self.display_name),
            )
            .field("label", &redacted_optional_wallet_debug_value(&self.label))
            .field("tag", &redacted_optional_wallet_debug_value(&self.tag))
            .field("verified", &self.verified)
            .field("copy_score", &self.copy_score)
            .field("size", &self.size)
            .field("notional_size", &self.notional_size)
            .field("entry_price", &self.entry_price)
            .field("liquidation_price", &self.liquidation_price)
            .field("unrealized_pnl", &self.unrealized_pnl)
            .field("funding_pnl", &self.funding_pnl)
            .field("account_value", &self.account_value)
            .finish()
    }
}

/// Aggregated HyperDash positioning for one perp ticker.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TickerPositions {
    pub coin: String,
    pub positions: Vec<TickerPositionEntry>,
    #[serde(rename = "totalLongNotional")]
    pub total_long_notional: f64,
    #[serde(rename = "totalShortNotional")]
    pub total_short_notional: f64,
    #[serde(rename = "totalNotional")]
    pub total_notional: f64,
    #[serde(rename = "longCount")]
    pub long_count: u64,
    #[serde(rename = "shortCount")]
    pub short_count: u64,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    pub timestamp: String,
}

/// A single wallet-level position delta from the HyperDash perp delta endpoint.
#[derive(Clone, Deserialize, PartialEq)]
pub struct PerpDeltaEntry {
    pub address: String,
    pub current: f64,
    pub delta: f64,
}

impl fmt::Debug for PerpDeltaEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PerpDeltaEntry")
            .field("address", &"<redacted>")
            .field("current", &self.current)
            .field("delta", &self.delta)
            .finish()
    }
}

/// Position-size changes for one perp market and timeframe.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PerpDeltas {
    pub market: String,
    pub timeframe: String,
    pub deltas: Vec<PerpDeltaEntry>,
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

#[derive(Deserialize)]
pub(super) struct GqlError {
    pub(super) message: String,
}

impl fmt::Debug for GqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = redact_sensitive_response_text(&self.message);
        f.debug_struct("GqlError")
            .field("message", &message)
            .finish()
    }
}

fn redacted_optional_wallet_debug_value(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(redacted_wallet_debug_value)
}

fn redacted_wallet_debug_value(value: &str) -> &str {
    let value = value.trim();
    let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    else {
        return value;
    };
    if hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        "<redacted>"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{GqlError, PerpDeltaEntry, PerpDeltas, TickerPositionEntry, TickerPositions};

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn ticker_positions_debug_redacts_wallet_addresses() {
        let positions = TickerPositions {
            coin: "HYPE".to_string(),
            positions: vec![TickerPositionEntry {
                address: TEST_ADDRESS.to_string(),
                display_name: Some(TEST_ADDRESS.to_string()),
                label: Some("Whale".to_string()),
                tag: Some(TEST_ADDRESS.to_string()),
                verified: Some(true),
                copy_score: Some(61.5),
                size: 12.5,
                notional_size: 500.25,
                entry_price: 30.0,
                liquidation_price: None,
                unrealized_pnl: 125.75,
                funding_pnl: -4.5,
                account_value: 1000.0,
            }],
            total_long_notional: 600.0,
            total_short_notional: 400.0,
            total_notional: 1000.0,
            long_count: 3,
            short_count: 2,
            total_count: 5,
            has_more: true,
            timestamp: "2026-05-18T11:52:39.585Z".to_string(),
        };

        let rendered = format!("{positions:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("Whale"));
        assert!(!rendered.contains(TEST_ADDRESS));
    }

    #[test]
    fn perp_deltas_debug_redacts_wallet_addresses() {
        let deltas = PerpDeltas {
            market: "HYPE".to_string(),
            timeframe: "15m".to_string(),
            deltas: vec![PerpDeltaEntry {
                address: TEST_ADDRESS.to_string(),
                current: -25.5,
                delta: 10.25,
            }],
        };

        let rendered = format!("{deltas:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ADDRESS));
    }

    #[test]
    fn graphql_error_debug_redacts_sensitive_values() {
        let error = GqlError {
            message: "provider echoed api_key=\"hyper-secret\" Bearer bearer-secret trace=0x0123456789abcdef0123456789abcdef01234567"
                .to_string(),
        };

        let rendered = format!("{error:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("<redacted-hex>"));
        for secret in [
            "hyper-secret",
            "bearer-secret",
            "0123456789abcdef0123456789abcdef01234567",
        ] {
            assert!(!rendered.contains(secret), "debug output leaked {secret}");
        }
    }
}
