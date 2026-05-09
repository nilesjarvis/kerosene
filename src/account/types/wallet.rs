use super::{AssetPosition, ClearinghouseState, OpenOrder, SpotClearinghouseState};

// ---------------------------------------------------------------------------
// Wallet Tracker Models
// ---------------------------------------------------------------------------

/// Lightweight snapshot used by the wallet tracker window.
#[derive(Debug, Clone)]
pub struct WalletTrackerSnapshot {
    pub equity: Option<f64>,
    pub withdrawable: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub margin_used_pct: Option<f64>,
    pub open_trade_count: Option<usize>,
    pub open_order_count: usize,
    pub long_exposure: Option<f64>,
    pub short_exposure: Option<f64>,
}

/// Per-position row for a detailed watched wallet view.
#[derive(Debug, Clone)]
pub struct WalletPositionDetail {
    /// Empty string = main perp dex. Non-empty = HIP-3 dex name.
    pub dex: String,
    pub asset_position: AssetPosition,
}

/// Per-order row for a detailed watched wallet view.
#[derive(Debug, Clone)]
pub struct WalletOpenOrderDetail {
    /// Empty string = main perp dex. Non-empty = HIP-3 dex name.
    pub dex: String,
    pub order: OpenOrder,
}

/// Full watch-only wallet snapshot used by detachable wallet-detail windows.
#[derive(Debug, Clone)]
pub struct WalletDetailsData {
    pub clearinghouse: ClearinghouseState,
    pub spot: SpotClearinghouseState,
    pub positions: Vec<WalletPositionDetail>,
    pub open_orders: Vec<WalletOpenOrderDetail>,
    pub warnings: Vec<String>,
    pub fetched_at_ms: u64,
}
