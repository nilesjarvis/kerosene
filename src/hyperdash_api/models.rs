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
                &self.display_name.as_ref().map(|_| "<redacted>"),
            )
            .field("label", &self.label.as_ref().map(|_| "<redacted>"))
            .field("tag", &self.tag.as_ref().map(|_| "<redacted>"))
            .field("verified", &self.verified.map(|_| "<redacted>"))
            .field("copy_score", &self.copy_score.map(|_| "<redacted>"))
            .field("size", &"<redacted>")
            .field("notional_size", &"<redacted>")
            .field("entry_price", &"<redacted>")
            .field(
                "liquidation_price",
                &self.liquidation_price.map(|_| "<redacted>"),
            )
            .field("unrealized_pnl", &"<redacted>")
            .field("funding_pnl", &"<redacted>")
            .field("account_value", &"<redacted>")
            .finish()
    }
}

/// Aggregated HyperDash positioning for one perp ticker.
#[derive(Clone, Deserialize, PartialEq)]
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

impl fmt::Debug for TickerPositions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TickerPositions")
            .field("coin", &self.coin)
            .field(
                "positions",
                &format_args!("<{} redacted>", self.positions.len()),
            )
            .field("total_long_notional", &self.total_long_notional)
            .field("total_short_notional", &self.total_short_notional)
            .field("total_notional", &self.total_notional)
            .field("long_count", &self.long_count)
            .field("short_count", &self.short_count)
            .field("total_count", &self.total_count)
            .field("has_more", &self.has_more)
            .field("timestamp", &self.timestamp)
            .finish()
    }
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
            .field("current", &"<redacted>")
            .field("delta", &"<redacted>")
            .finish()
    }
}

/// Position-size changes for one perp market and timeframe.
#[derive(Clone, Deserialize, PartialEq)]
pub struct PerpDeltas {
    pub market: String,
    pub timeframe: String,
    pub deltas: Vec<PerpDeltaEntry>,
}

impl fmt::Debug for PerpDeltas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PerpDeltas")
            .field("market", &self.market)
            .field("timeframe", &self.timeframe)
            .field("deltas", &format_args!("<{} redacted>", self.deltas.len()))
            .finish()
    }
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

#[cfg(test)]
mod tests {
    use super::{GqlError, PerpDeltaEntry, PerpDeltas, TickerPositionEntry, TickerPositions};

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn ticker_positions_debug_separates_public_aggregates_from_wallet_rows() {
        const DISPLAY_NAME: &str = "private-hyperdash-display-name-sentinel";
        const LABEL: &str = "private-hyperdash-label-sentinel";
        const TAG: &str = "private-hyperdash-tag-sentinel";
        const COPY_SCORE: f64 = 201_910.937_5;
        const SIZE: f64 = 918_273.125;
        const NOTIONAL_SIZE: f64 = 827_364.25;
        const ENTRY_PRICE: f64 = 736_455.375;
        const LIQUIDATION_PRICE: f64 = 645_546.5;
        const UNREALIZED_PNL: f64 = 534_637.625;
        const FUNDING_PNL: f64 = -423_728.75;
        const ACCOUNT_VALUE: f64 = 312_819.875;
        let positions = TickerPositions {
            coin: "HYPE".to_string(),
            positions: vec![TickerPositionEntry {
                address: TEST_ADDRESS.to_string(),
                display_name: Some(DISPLAY_NAME.to_string()),
                label: Some(LABEL.to_string()),
                tag: Some(TAG.to_string()),
                verified: Some(true),
                copy_score: Some(COPY_SCORE),
                size: SIZE,
                notional_size: NOTIONAL_SIZE,
                entry_price: ENTRY_PRICE,
                liquidation_price: Some(LIQUIDATION_PRICE),
                unrealized_pnl: UNREALIZED_PNL,
                funding_pnl: FUNDING_PNL,
                account_value: ACCOUNT_VALUE,
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
        let value_before = positions.clone();

        let parent_debug = format!("{positions:?}");
        let entry_debug = format!("{:?}", positions.positions[0]);

        assert!(parent_debug.contains("coin: \"HYPE\""), "{parent_debug}");
        assert!(
            parent_debug.contains("positions: <1 redacted>"),
            "{parent_debug}"
        );
        assert!(
            parent_debug.contains("total_long_notional: 600.0"),
            "{parent_debug}"
        );
        assert!(parent_debug.contains("has_more: true"), "{parent_debug}");
        assert!(entry_debug.contains("<redacted>"), "{entry_debug}");
        assert!(
            !entry_debug.contains("verified: Some(true)"),
            "{entry_debug}"
        );
        for sensitive in [TEST_ADDRESS, DISPLAY_NAME, LABEL, TAG] {
            assert!(
                !parent_debug.contains(sensitive),
                "{sensitive} leaked in {parent_debug}"
            );
            assert!(
                !entry_debug.contains(sensitive),
                "{sensitive} leaked in {entry_debug}"
            );
        }
        for sensitive in [
            COPY_SCORE,
            SIZE,
            NOTIONAL_SIZE,
            ENTRY_PRICE,
            LIQUIDATION_PRICE,
            UNREALIZED_PNL,
            FUNDING_PNL,
            ACCOUNT_VALUE,
        ] {
            let sensitive = format!("{sensitive:?}");
            assert!(
                !parent_debug.contains(&sensitive),
                "{sensitive} leaked in {parent_debug}"
            );
            assert!(
                !entry_debug.contains(&sensitive),
                "{sensitive} leaked in {entry_debug}"
            );
        }
        assert_eq!(positions, value_before);
    }

    #[test]
    fn perp_deltas_debug_keeps_public_context_and_redacts_wallet_values() {
        const CURRENT: f64 = -918_273.125;
        const DELTA: f64 = 827_364.25;
        let deltas = PerpDeltas {
            market: "HYPE".to_string(),
            timeframe: "15m".to_string(),
            deltas: vec![PerpDeltaEntry {
                address: TEST_ADDRESS.to_string(),
                current: CURRENT,
                delta: DELTA,
            }],
        };
        let value_before = deltas.clone();

        let parent_debug = format!("{deltas:?}");
        let entry_debug = format!("{:?}", deltas.deltas[0]);

        assert!(parent_debug.contains("market: \"HYPE\""), "{parent_debug}");
        assert!(
            parent_debug.contains("timeframe: \"15m\""),
            "{parent_debug}"
        );
        assert!(
            parent_debug.contains("deltas: <1 redacted>"),
            "{parent_debug}"
        );
        assert!(entry_debug.contains("<redacted>"), "{entry_debug}");
        for sensitive in [
            TEST_ADDRESS.to_string(),
            format!("{CURRENT:?}"),
            format!("{DELTA:?}"),
        ] {
            assert!(
                !parent_debug.contains(&sensitive),
                "{sensitive} leaked in {parent_debug}"
            );
            assert!(
                !entry_debug.contains(&sensitive),
                "{sensitive} leaked in {entry_debug}"
            );
        }
        assert_eq!(deltas, value_before);
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
