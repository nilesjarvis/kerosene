use super::{AssetPosition, ClearinghouseState, OpenOrder, SpotClearinghouseState};
use std::fmt;

// ---------------------------------------------------------------------------
// Wallet Tracker Models
// ---------------------------------------------------------------------------

/// Lightweight snapshot used by the wallet tracker window.
#[derive(Clone)]
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

impl fmt::Debug for WalletTrackerSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletTrackerSnapshot")
            .field("equity", &redacted_presence(&self.equity))
            .field("withdrawable", &redacted_presence(&self.withdrawable))
            .field("unrealized_pnl", &redacted_presence(&self.unrealized_pnl))
            .field("margin_used_pct", &redacted_presence(&self.margin_used_pct))
            .field("open_trade_count", &self.open_trade_count)
            .field("open_order_count", &self.open_order_count)
            .field("long_exposure", &redacted_presence(&self.long_exposure))
            .field("short_exposure", &redacted_presence(&self.short_exposure))
            .finish()
    }
}

/// Per-position row for a detailed watched wallet view.
#[derive(Clone)]
pub struct WalletPositionDetail {
    /// Empty string = main perp dex. Non-empty = HIP-3 dex name.
    pub dex: String,
    pub asset_position: AssetPosition,
}

impl fmt::Debug for WalletPositionDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletPositionDetail")
            .field("dex", &self.dex)
            .field("asset_position", &format_args!("<redacted>"))
            .finish()
    }
}

/// Per-order row for a detailed watched wallet view.
#[derive(Clone)]
pub struct WalletOpenOrderDetail {
    /// Empty string = main perp dex. Non-empty = HIP-3 dex name.
    pub dex: String,
    pub order: OpenOrder,
}

impl fmt::Debug for WalletOpenOrderDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletOpenOrderDetail")
            .field("dex", &self.dex)
            .field("order", &format_args!("<redacted>"))
            .finish()
    }
}

/// Full watch-only wallet snapshot used by detachable wallet-detail windows.
#[derive(Clone)]
pub struct WalletDetailsData {
    pub clearinghouse: ClearinghouseState,
    pub spot: SpotClearinghouseState,
    pub positions: Vec<WalletPositionDetail>,
    pub open_orders: Vec<WalletOpenOrderDetail>,
    pub warnings: Vec<String>,
    pub fetched_at_ms: u64,
}

impl fmt::Debug for WalletDetailsData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletDetailsData")
            .field(
                "clearinghouse",
                &format_args!("positions_len={}", self.clearinghouse.asset_positions.len()),
            )
            .field(
                "spot",
                &format_args!("balances_len={}", self.spot.balances.len()),
            )
            .field("positions", &format_args!("len={}", self.positions.len()))
            .field(
                "open_orders",
                &format_args!("len={}", self.open_orders.len()),
            )
            .field("warnings", &format_args!("len={}", self.warnings.len()))
            .field("fetched_at_ms", &self.fetched_at_ms)
            .finish()
    }
}

fn redacted_presence<T>(value: &Option<T>) -> Option<&'static str> {
    value.as_ref().map(|_| "<redacted>")
}

#[cfg(test)]
mod tests {
    use super::{
        AssetPosition, ClearinghouseState, OpenOrder, SpotClearinghouseState, WalletDetailsData,
        WalletOpenOrderDetail, WalletPositionDetail, WalletTrackerSnapshot,
    };
    use crate::account::{MarginSummary, Position, PositionLeverage, SpotBalance};

    #[test]
    fn wallet_tracker_snapshot_debug_redacts_financial_values() {
        let snapshot = WalletTrackerSnapshot {
            equity: Some(987654321.123),
            withdrawable: Some(12345.67),
            unrealized_pnl: Some(-7654.321),
            margin_used_pct: Some(42.42),
            open_trade_count: Some(3),
            open_order_count: 5,
            long_exposure: Some(1111.0),
            short_exposure: Some(2222.0),
        };

        let rendered = format!("{snapshot:?}");

        assert!(rendered.contains("equity: Some(\"<redacted>\")"));
        assert!(rendered.contains("open_trade_count: Some(3)"));
        assert!(rendered.contains("open_order_count: 5"));
        for secret in ["987654321.123", "12345.67", "-7654.321", "42.42"] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    #[test]
    fn wallet_detail_row_debug_redacts_position_and_order_payloads() {
        let position_detail = WalletPositionDetail {
            dex: "xyz".to_string(),
            asset_position: asset_position("POSITIONSECRET", "position-secret-value"),
        };
        let order_detail = WalletOpenOrderDetail {
            dex: "xyz".to_string(),
            order: open_order("ORDERSECRET", "order-secret-price"),
        };

        let rendered_position = format!("{position_detail:?}");
        let rendered_order = format!("{order_detail:?}");

        assert!(rendered_position.contains("asset_position: <redacted>"));
        assert!(rendered_order.contains("order: <redacted>"));
        assert!(!rendered_position.contains("POSITIONSECRET"));
        assert!(!rendered_position.contains("position-secret-value"));
        assert!(!rendered_order.contains("ORDERSECRET"));
        assert!(!rendered_order.contains("order-secret-price"));
    }

    #[test]
    fn wallet_details_data_debug_summarizes_payloads() {
        let data = WalletDetailsData {
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "wallet-secret-equity".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "wallet-secret-withdrawable".to_string(),
                asset_positions: vec![asset_position("CLEARINGSECRET", "clearing-secret-value")],
            },
            spot: SpotClearinghouseState {
                balances: vec![SpotBalance {
                    coin: "SPOTSECRET".to_string(),
                    token: Some(1),
                    total: "spot-secret-total".to_string(),
                    hold: "0".to_string(),
                    entry_ntl: "0".to_string(),
                    supplied: None,
                }],
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            positions: vec![WalletPositionDetail {
                dex: "xyz".to_string(),
                asset_position: asset_position("DETAILSECRET", "detail-secret-value"),
            }],
            open_orders: vec![WalletOpenOrderDetail {
                dex: "xyz".to_string(),
                order: open_order("ORDERSECRET", "order-secret-price"),
            }],
            warnings: vec!["wallet-secret-warning".to_string()],
            fetched_at_ms: 42,
        };

        let rendered = format!("{data:?}");

        assert!(rendered.contains("clearinghouse: positions_len=1"));
        assert!(rendered.contains("spot: balances_len=1"));
        assert!(rendered.contains("positions: len=1"));
        assert!(rendered.contains("open_orders: len=1"));
        assert!(rendered.contains("warnings: len=1"));
        for secret in [
            "wallet-secret-equity",
            "wallet-secret-withdrawable",
            "CLEARINGSECRET",
            "clearing-secret-value",
            "SPOTSECRET",
            "spot-secret-total",
            "DETAILSECRET",
            "detail-secret-value",
            "ORDERSECRET",
            "order-secret-price",
            "wallet-secret-warning",
        ] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }

    fn asset_position(coin: &str, position_value: &str) -> AssetPosition {
        AssetPosition {
            position: Position {
                coin: coin.to_string(),
                szi: "1".to_string(),
                entry_px: "100".to_string(),
                position_value: position_value.to_string(),
                unrealized_pnl: "0".to_string(),
                liquidation_px: None,
                leverage: PositionLeverage {
                    leverage_type: "cross".to_string(),
                    value: 3,
                },
                margin_used: "0".to_string(),
                cum_funding: None,
            },
            liquidation_px: None,
        }
    }

    fn open_order(coin: &str, limit_px: &str) -> OpenOrder {
        OpenOrder {
            coin: coin.to_string(),
            side: "B".to_string(),
            limit_px: limit_px.to_string(),
            sz: "1".to_string(),
            oid: 42,
            timestamp: 7,
            reduce_only: None,
            is_trigger: None,
            order_type: None,
            tif: None,
            trigger_px: None,
        }
    }
}
