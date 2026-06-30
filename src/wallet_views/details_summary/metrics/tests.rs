use super::*;
use crate::account::{
    AssetPosition, ClearinghouseState, MarginSummary, Position, PositionLeverage, SpotBalance,
    SpotClearinghouseState, UserFill, WalletDetailsData, WalletPositionDetail,
};
use crate::api::{ExchangeSymbol, MarketType};
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
fn wallet_details_summary_includes_reconciled_spot_fill_cost_basis() {
    let mut terminal = TradingTerminal::boot().0;
    let now_ms = TradingTerminal::now_ms();
    terminal.exchange_symbols = vec![spot_symbol("@142", "UBTC", 10_142)];
    terminal.all_mids.insert("@142".to_string(), 58_358.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@142".to_string(), now_ms);

    let metrics = terminal.wallet_details_summary_metrics(&wallet_details_with_spot_basis(now_ms));

    assert_eq!(metrics.active_position_count, 1);
    assert_eq!(metrics.short_exposure, Some(0.0));
    assert_approx_eq(metrics.long_exposure, 6.7491729032 * 58_358.0);
    assert_approx_eq(
        metrics.unrealized_pnl,
        6.7491729032 * 58_358.0 - (60_191.0 + 58_395.0 * 5.753),
    );
}

fn wallet_details_with_spot_basis(now_ms: u64) -> WalletDetailsData {
    WalletDetailsData {
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "800".to_string(),
            asset_positions: Vec::new(),
        },
        spot: SpotClearinghouseState {
            balances: vec![SpotBalance {
                coin: "UBTC".to_string(),
                token: Some(197),
                total: "6.7491729032".to_string(),
                hold: "0".to_string(),
                entry_ntl: "0".to_string(),
                supplied: Some("6.7491729032".to_string()),
            }],
            portfolio_margin_enabled: true,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        positions: Vec::new(),
        open_orders: Vec::new(),
        fills: vec![
            spot_fill("@142", "60191", "1.0", "0.0004", "UBTC", 1),
            spot_fill("@142", "58395", "5.753", "0.0034270968", "UBTC", 2),
        ],
        warnings: Vec::new(),
        fetched_at_ms: now_ms,
    }
}

fn spot_symbol(key: &str, ticker: &str, asset_index: u32) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        keywords: Vec::new(),
        asset_index,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn spot_fill(coin: &str, px: &str, sz: &str, fee: &str, fee_token: &str, time: u64) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: px.to_string(),
        sz: sz.to_string(),
        side: "B".to_string(),
        time,
        hash: None,
        tid: Some(time),
        oid: None,
        dir: "Buy".to_string(),
        closed_pnl: "0".to_string(),
        fee: fee.to_string(),
        fee_token: Some(fee_token.to_string()),
    }
}

fn assert_approx_eq(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("metric");
    assert!(
        (actual - expected).abs() <= expected.abs().max(1.0) * 1e-10,
        "expected {expected}, got {actual}"
    );
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
        fills: Vec::new(),
        warnings: Vec::new(),
        fetched_at_ms: now_ms,
    });

    assert_eq!(metrics.active_position_count, 1);
    assert_eq!(metrics.long_exposure, Some(0.0));
    assert_eq!(metrics.short_exposure, Some(180.0));
    assert_eq!(metrics.unrealized_pnl, Some(20.0));
}
