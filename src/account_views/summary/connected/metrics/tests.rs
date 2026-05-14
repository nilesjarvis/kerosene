use super::*;
use crate::account::{
    AccountAbstractionMode, AccountData, ClearinghouseState, MarginSummary, SpotBalance,
    SpotClearinghouseState,
};
use crate::config::MarketUniverseConfig;

#[test]
fn summary_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_summary_number(" 42.5 "), Some(42.5));
    assert_eq!(parse_summary_number("-1.25"), Some(-1.25));

    assert_eq!(parse_summary_number("bad"), None);
    assert_eq!(parse_summary_number("NaN"), None);
    assert_eq!(parse_summary_number("inf"), None);
}

#[test]
fn summary_position_upnl_uses_live_mid_only_with_valid_inputs() {
    assert_eq!(position_upnl_value("2", "90", "1", Some(100.0)), Some(20.0));
    assert_eq!(
        position_upnl_value("bad", "90", "1", Some(100.0)),
        Some(1.0)
    );
    assert_eq!(position_upnl_value("bad", "90", "bad", Some(100.0)), None);
}

#[test]
fn summary_position_notional_uses_abs_live_mid_value() {
    assert_eq!(position_notional_value("-2", "1", Some(100.0)), Some(200.0));
    assert_eq!(
        position_notional_value("bad", "-123", Some(100.0)),
        Some(123.0)
    );
    assert_eq!(position_notional_value("bad", "bad", Some(100.0)), None);
}

#[test]
fn summary_effective_leverage_uses_notional_over_account_value() {
    assert_eq!(effective_leverage(Some(250.0), Some(100.0)), Some(2.5));
    assert_eq!(effective_leverage(Some(-250.0), Some(100.0)), Some(2.5));
    assert_eq!(leverage_string(Some(2.5)), "2.50x");
}

#[test]
fn summary_effective_leverage_handles_flat_or_invalid_equity() {
    assert_eq!(effective_leverage(Some(0.0), Some(0.0)), Some(0.0));
    assert_eq!(effective_leverage(Some(10.0), Some(0.0)), None);
    assert_eq!(effective_leverage(Some(10.0), Some(-5.0)), None);
    assert_eq!(effective_leverage(None, Some(100.0)), None);
    assert_eq!(leverage_string(None), "Invalid data");
}

#[test]
fn summary_spot_value_does_not_zero_invalid_balances() {
    assert_eq!(spot_balance_value("USDC", "10", "0", None), Some(10.0));
    assert_eq!(spot_balance_value("PURR", "2", "3", Some(4.0)), Some(8.0));
    assert_eq!(spot_balance_value("PURR", "2", "3", None), Some(3.0));
    assert_eq!(spot_balance_value("PURR", "bad", "3", Some(4.0)), None);
    assert_eq!(spot_balance_value("PURR", "2", "bad", None), None);
}

#[test]
fn summary_percent_string_rejects_invalid_margin_ratio() {
    assert_eq!(summary_percent_string(Some(0.125)), "12.50%");
    assert_eq!(summary_percent_string(None), "Invalid data");
}

fn account_data_for_shared_total(
    abstraction: AccountAbstractionMode,
    portfolio_margin_enabled: bool,
) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: abstraction,
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "92".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: vec![SpotBalance {
                coin: "USDC".to_string(),
                token: Some(0),
                total: "100".to_string(),
                hold: "0".to_string(),
                entry_ntl: "0".to_string(),
                supplied: None,
            }],
            portfolio_margin_enabled,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 1,
    }
}

#[test]
fn portfolio_margin_shared_total_prefers_spot_portfolio_value() {
    let data = account_data_for_shared_total(AccountAbstractionMode::PortfolioMargin, true);

    assert_eq!(
        shared_account_total_value(&data, || Some(100.0)),
        Some(100.0)
    );
}

#[test]
fn shared_non_portfolio_total_uses_spot_fallback_when_larger() {
    let data = account_data_for_shared_total(AccountAbstractionMode::DexAbstraction, false);

    assert_eq!(
        shared_account_total_value(&data, || Some(100.0)),
        Some(100.0)
    );
}

#[test]
fn shared_token_total_uses_selected_collateral_balance() {
    let mut data = account_data_for_shared_total(AccountAbstractionMode::DexAbstraction, false);
    data.spot.balances.push(SpotBalance {
        coin: "XYZC".to_string(),
        token: Some(404),
        total: "2".to_string(),
        hold: "0".to_string(),
        entry_ntl: "25".to_string(),
        supplied: None,
    });

    assert_eq!(
        shared_account_token_total_value(&data, 404, |_| Some(15.0)),
        Some(30.0)
    );
}

#[test]
fn shared_token_total_fails_closed_without_collateral_balance() {
    let data = account_data_for_shared_total(AccountAbstractionMode::DexAbstraction, false);

    assert_eq!(
        shared_account_token_total_value(&data, 404, |_| Some(15.0)),
        None
    );
}

#[test]
fn portfolio_margin_summary_keeps_spot_value_in_selected_hip3_universe() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    terminal.muted_tickers.clear();
    let data = account_data_for_shared_total(AccountAbstractionMode::PortfolioMargin, true);

    let values = terminal.connected_summary_values(&data);

    assert_eq!(values.total_value, "100.00");
    assert_eq!(values.available, Some(100.0));
    assert_eq!(values.available_value, "100.00");
}
