use super::*;
use crate::account::{
    AssetPosition, ClearinghouseState, MarginSummary, Position, PositionLeverage,
    SpotClearinghouseState, WalletDetailsData, WalletPositionDetail,
};
use crate::app_state::TradingTerminal;

#[test]
fn wallet_margin_pct_rejects_invalid_or_ambiguous_inputs() {
    assert_eq!(wallet_margin_pct(Some(100.0), Some(25.0)), Some(25.0));
    assert_eq!(wallet_margin_pct(Some(0.0), Some(0.0)), Some(0.0));
    assert_eq!(wallet_margin_pct(Some(0.0), Some(1.0)), None);
    assert_eq!(wallet_margin_pct(None, Some(1.0)), None);
    assert_eq!(wallet_margin_pct(Some(100.0), None), None);
}

#[test]
fn wallet_details_summary_prefers_live_mark_for_position_metrics() {
    let mut terminal = TradingTerminal::boot().0;
    let now_ms = TradingTerminal::now_ms();
    terminal.all_mids.insert("BTC".to_string(), 90.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), now_ms);

    let metrics = terminal.wallet_details_summary_metrics(&WalletDetailsData {
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "999".to_string(),
                total_margin_used: "100".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "800".to_string(),
            asset_positions: Vec::new(),
        },
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        positions: vec![WalletPositionDetail {
            dex: String::new(),
            asset_position: AssetPosition {
                position: Position {
                    coin: "BTC".to_string(),
                    szi: "-2".to_string(),
                    entry_px: "100".to_string(),
                    position_value: "999".to_string(),
                    unrealized_pnl: "-999".to_string(),
                    liquidation_px: None,
                    leverage: PositionLeverage {
                        leverage_type: "cross".to_string(),
                        value: 10,
                    },
                    margin_used: "0".to_string(),
                    cum_funding: None,
                },
                liquidation_px: None,
            },
        }],
        open_orders: Vec::new(),
        warnings: Vec::new(),
        fetched_at_ms: now_ms,
    });

    assert_eq!(metrics.active_position_count, 1);
    assert_eq!(metrics.long_exposure, Some(0.0));
    assert_eq!(metrics.short_exposure, Some(180.0));
    assert_eq!(metrics.unrealized_pnl, Some(20.0));
}
