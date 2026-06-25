use serde::Deserialize;

use std::{collections::HashSet, fmt};

use crate::helpers::parse_positive_finite_number;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Account Wire Models
// ---------------------------------------------------------------------------

/// How Hyperliquid currently abstracts a user's spot/perp balances.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AccountAbstractionMode {
    Disabled,
    #[default]
    Default,
    UnifiedAccount,
    PortfolioMargin,
    DexAbstraction,
    Unknown(String),
}

impl AccountAbstractionMode {
    pub fn from_api_value(raw: &str) -> Self {
        match raw {
            "disabled" => Self::Disabled,
            "default" => Self::Default,
            "unifiedAccount" => Self::UnifiedAccount,
            "portfolioMargin" => Self::PortfolioMargin,
            "dexAbstraction" => Self::DexAbstraction,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn uses_shared_account_balance(&self) -> bool {
        matches!(
            self,
            Self::Default | Self::UnifiedAccount | Self::PortfolioMargin | Self::DexAbstraction
        )
    }
}

/// Margin summary from clearinghouseState.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginSummary {
    pub account_value: String,
    pub total_ntl_pos: String,
    pub total_margin_used: String,
}

impl fmt::Debug for MarginSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarginSummary")
            .field("account_value", &"<redacted>")
            .field("total_ntl_pos", &"<redacted>")
            .field("total_margin_used", &"<redacted>")
            .finish()
    }
}

/// Cumulative funding amounts for a position.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CumFunding {
    /// Funding since the current position was opened.
    pub since_open: String,
}

impl fmt::Debug for CumFunding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CumFunding")
            .field("since_open", &"<redacted>")
            .finish()
    }
}

/// A single perpetual position.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub coin: String,
    pub szi: String,
    pub entry_px: String,
    pub position_value: String,
    pub unrealized_pnl: String,
    pub liquidation_px: Option<String>,
    pub leverage: PositionLeverage,
    #[serde(default)]
    pub margin_used: String,
    /// Raw cumulative funding from `clearinghouseState`.
    /// Invert this value when displaying wallet-balance PnL.
    #[serde(default)]
    pub cum_funding: Option<CumFunding>,
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Position")
            .field("coin", &"<redacted>")
            .field("szi", &"<redacted>")
            .field("entry_px", &"<redacted>")
            .field("position_value", &"<redacted>")
            .field("unrealized_pnl", &"<redacted>")
            .field("has_liquidation_px", &self.liquidation_px.is_some())
            .field("leverage", &self.leverage)
            .field("margin_used", &"<redacted>")
            .field("has_cum_funding", &self.cum_funding.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PositionLeverage {
    #[serde(rename = "type")]
    pub leverage_type: String,
    pub value: u32,
}

/// Wrapper for the position in assetPositions array.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPosition {
    pub position: Position,
    /// Some API responses expose liquidation price at the wrapper level.
    #[serde(default)]
    pub liquidation_px: Option<String>,
}

impl fmt::Debug for AssetPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetPosition")
            .field("position", &self.position)
            .field("has_liquidation_px", &self.liquidation_px.is_some())
            .finish()
    }
}

/// Full clearinghouse state for a user (perps).
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearinghouseState {
    pub margin_summary: MarginSummary,
    /// Cross-margin summary (separate from isolated positions).
    pub cross_margin_summary: Option<MarginSummary>,
    /// Cross-margin maintenance margin used.
    pub cross_maintenance_margin_used: Option<String>,
    pub withdrawable: String,
    pub asset_positions: Vec<AssetPosition>,
}

impl fmt::Debug for ClearinghouseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClearinghouseState")
            .field("margin_summary", &self.margin_summary)
            .field(
                "has_cross_margin_summary",
                &self.cross_margin_summary.is_some(),
            )
            .field(
                "has_cross_maintenance_margin_used",
                &self.cross_maintenance_margin_used.is_some(),
            )
            .field("withdrawable", &"<redacted>")
            .field("asset_positions_count", &self.asset_positions.len())
            .finish()
    }
}

/// Real-time asset context for a market (perp or spot).
/// For perp: all fields populated. For spot: funding, open_interest,
/// oracle_px and mark_px will be None.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetContext {
    #[serde(default)]
    pub funding: Option<String>,
    #[serde(default)]
    pub open_interest: Option<String>,
    #[serde(default)]
    pub oracle_px: Option<String>,
    #[serde(default)]
    pub mark_px: Option<String>,
    #[serde(default)]
    pub mid_px: Option<String>,
    #[serde(default)]
    pub prev_day_px: Option<String>,
    #[serde(default)]
    pub day_ntl_vlm: Option<String>,
    #[serde(default)]
    pub day_base_vlm: Option<String>,
    #[serde(default, rename = "impactPxs")]
    pub impact_pxs: Option<Vec<String>>,
}

impl fmt::Debug for AssetContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetContext")
            .field("has_funding", &self.funding.is_some())
            .field("has_open_interest", &self.open_interest.is_some())
            .field("has_oracle_px", &self.oracle_px.is_some())
            .field("has_mark_px", &self.mark_px.is_some())
            .field("has_mid_px", &self.mid_px.is_some())
            .field("has_prev_day_px", &self.prev_day_px.is_some())
            .field("has_day_ntl_vlm", &self.day_ntl_vlm.is_some())
            .field("has_day_base_vlm", &self.day_base_vlm.is_some())
            .field("impact_pxs_count", &self.impact_pxs.as_ref().map(Vec::len))
            .finish()
    }
}

impl AssetContext {
    /// Live tradable price for account-side mark-to-market calculations.
    /// Hydromancer tick payloads normally carry `midPx`; `markPx` is retained
    /// as a fallback for compatible asset-context payloads.
    pub(crate) fn live_price(&self) -> Option<f64> {
        self.mid_px
            .as_deref()
            .and_then(parse_positive_finite_number)
            .or_else(|| {
                self.mark_px
                    .as_deref()
                    .and_then(parse_positive_finite_number)
            })
    }

    /// Bid/ask spread derived from `impact_pxs` (`[bid, ask]`).
    /// Returns `None` when impact prices are missing, unparseable, or crossed.
    pub(crate) fn impact_spread(&self) -> Option<f64> {
        let impact = self.impact_pxs.as_deref()?;
        if impact.len() < 2 {
            return None;
        }

        let bid = impact[0].parse::<f64>().ok()?;
        let ask = impact[1].parse::<f64>().ok()?;
        let spread = ask - bid;
        (spread.is_finite() && spread >= 0.0).then_some(spread)
    }
}

/// A user's open order.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrder {
    pub coin: String,
    pub side: String, // "A" (ask/sell) or "B" (bid/buy)
    pub limit_px: String,
    pub sz: String,
    pub oid: u64,
    pub timestamp: u64,
    #[serde(default)]
    pub reduce_only: Option<bool>,
    #[serde(default, rename = "isTrigger")]
    pub is_trigger: Option<bool>,
    #[serde(default, rename = "orderType")]
    pub order_type: Option<String>,
    #[serde(default)]
    pub tif: Option<String>,
    #[serde(default, rename = "triggerPx")]
    pub trigger_px: Option<String>,
}

impl fmt::Debug for OpenOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenOrder")
            .field("coin", &"<redacted>")
            .field("side", &self.side)
            .field("limit_px", &"<redacted>")
            .field("sz", &"<redacted>")
            .field("oid", &"<redacted>")
            .field("timestamp", &self.timestamp)
            .field("reduce_only", &self.reduce_only)
            .field("is_trigger", &self.is_trigger)
            .field("order_type", &self.order_type)
            .field("tif", &self.tif)
            .field(
                "trigger_px",
                &self.trigger_px.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

/// A user's fill (trade).
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFill {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String, // "A" or "B"
    pub time: u64,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub tid: Option<u64>,
    #[serde(default)]
    pub oid: Option<u64>,
    pub dir: String, // "Open Long", "Close Short", etc.
    pub closed_pnl: String,
    pub fee: String,
}

impl fmt::Debug for UserFill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserFill")
            .field("coin", &"<redacted>")
            .field("px", &"<redacted>")
            .field("sz", &"<redacted>")
            .field("side", &self.side)
            .field("time", &self.time)
            .field("has_hash", &self.hash.is_some())
            .field("has_tid", &self.tid.is_some())
            .field("has_oid", &self.oid.is_some())
            .field("dir", &self.dir)
            .field("closed_pnl", &"<redacted>")
            .field("fee", &"<redacted>")
            .finish()
    }
}

impl UserFill {
    pub(crate) fn dedup_key(&self) -> String {
        if let Some(tid) = self.tid {
            return format!("tid:{tid}");
        }
        let fill_fields = format!(
            "{:?}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.oid,
            self.time,
            self.coin,
            self.px,
            self.sz,
            self.side,
            self.dir,
            self.closed_pnl,
            self.fee
        );
        if let Some(hash) = self.hash.as_deref().map(str::trim)
            && !hash.is_empty()
        {
            return format!("hash:{hash}\u{1f}{fill_fields}");
        }
        format!("fallback:{fill_fields}")
    }
}

pub(crate) fn dedupe_user_fills_preserving_order(fills: Vec<UserFill>) -> Vec<UserFill> {
    let mut seen = HashSet::with_capacity(fills.len());
    fills
        .into_iter()
        .filter(|fill| seen.insert(fill.dedup_key()))
        .collect()
}

/// A single funding payment record from the `userFunding` endpoint.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingDelta {
    pub coin: String,
    pub funding_rate: String,
    pub szi: String,
    /// USDC amount: negative = paid, positive = received.
    pub usdc: String,
}

impl fmt::Debug for FundingDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FundingDelta")
            .field("coin", &"<redacted>")
            .field("funding_rate", &"<redacted>")
            .field("szi", &"<redacted>")
            .field("usdc", &"<redacted>")
            .finish()
    }
}

/// A funding payment entry with timestamp.
#[derive(Clone, Deserialize)]
pub struct FundingEntry {
    pub delta: FundingDelta,
    pub time: u64,
}

impl fmt::Debug for FundingEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FundingEntry")
            .field("delta", &self.delta)
            .field("time", &self.time)
            .finish()
    }
}

/// A spot balance entry.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotBalance {
    pub coin: String,
    #[serde(default)]
    pub token: Option<u32>,
    pub total: String,
    pub hold: String,
    pub entry_ntl: String,
    /// Amount supplied to earn (if any).
    pub supplied: Option<String>,
}

impl fmt::Debug for SpotBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpotBalance")
            .field("coin", &"<redacted>")
            .field("token", &self.token)
            .field("total", &"<redacted>")
            .field("hold", &"<redacted>")
            .field("entry_ntl", &"<redacted>")
            .field("has_supplied", &self.supplied.is_some())
            .finish()
    }
}

/// Spot clearinghouse state.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotClearinghouseState {
    pub balances: Vec<SpotBalance>,
    /// Whether portfolio margin mode is enabled for this account.
    #[serde(default)]
    pub portfolio_margin_enabled: bool,
    /// Current portfolio margin ratio (0.0 = healthy, higher = riskier).
    #[serde(default)]
    pub portfolio_margin_ratio: Option<String>,
    /// Per-token available balance after maintenance margin.
    /// Format: `[(token_index, amount_string), ...]`.
    /// Token 0 = USDC. Only present for portfolio margin accounts.
    #[serde(default)]
    pub token_to_available_after_maintenance: Option<Vec<(u32, String)>>,
}

impl fmt::Debug for SpotClearinghouseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpotClearinghouseState")
            .field("balances_count", &self.balances.len())
            .field("portfolio_margin_enabled", &self.portfolio_margin_enabled)
            .field(
                "has_portfolio_margin_ratio",
                &self.portfolio_margin_ratio.is_some(),
            )
            .field(
                "token_to_available_after_maintenance_count",
                &self
                    .token_to_available_after_maintenance
                    .as_ref()
                    .map(Vec::len),
            )
            .finish()
    }
}
