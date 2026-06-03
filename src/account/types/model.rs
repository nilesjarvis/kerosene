use serde::Deserialize;

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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginSummary {
    pub account_value: String,
    pub total_ntl_pos: String,
    pub total_margin_used: String,
}

/// Cumulative funding amounts for a position.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CumFunding {
    /// Funding since the current position was opened.
    pub since_open: String,
}

/// A single perpetual position.
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct PositionLeverage {
    #[serde(rename = "type")]
    pub leverage_type: String,
    pub value: u32,
}

/// Wrapper for the position in assetPositions array.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPosition {
    pub position: Position,
    /// Some API responses expose liquidation price at the wrapper level.
    #[serde(default)]
    pub liquidation_px: Option<String>,
}

/// Full clearinghouse state for a user (perps).
#[derive(Debug, Clone, Deserialize)]
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

/// Real-time asset context for a market (perp or spot).
/// For perp: all fields populated. For spot: funding, open_interest,
/// oracle_px and mark_px will be None.
#[derive(Debug, Clone, Deserialize)]
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

/// A user's open order.
#[derive(Debug, Clone, Deserialize)]
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
}

/// A user's fill (trade).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFill {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String, // "A" or "B"
    pub time: u64,
    #[serde(default)]
    pub oid: Option<u64>,
    pub dir: String, // "Open Long", "Close Short", etc.
    pub closed_pnl: String,
    pub fee: String,
}

/// A single funding payment record from the `userFunding` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingDelta {
    pub coin: String,
    pub funding_rate: String,
    pub szi: String,
    /// USDC amount: negative = paid, positive = received.
    pub usdc: String,
}

/// A funding payment entry with timestamp.
#[derive(Debug, Clone, Deserialize)]
pub struct FundingEntry {
    pub delta: FundingDelta,
    pub time: u64,
}

/// A spot balance entry.
#[derive(Debug, Clone, Deserialize)]
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

/// Spot clearinghouse state.
#[derive(Debug, Clone, Deserialize)]
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
