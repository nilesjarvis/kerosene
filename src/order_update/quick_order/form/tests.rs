use super::{quick_order_quantity_for_percentage, toggled_quick_order_quantity_text};
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;

fn symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn account_data_without_positions() -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

#[test]
fn quick_order_percentage_quantity_formats_usd_and_coin() {
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 5_000_000.0, false, Some(100.0), 2),
        "12,500.00"
    );
}

#[test]
fn quick_order_percentage_quantity_rejects_invalid_inputs() {
    assert_eq!(
        quick_order_quantity_for_percentage(f32::NAN, 1_000.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 0.0, true, Some(100.0), 4),
        "0"
    );
    assert_eq!(
        quick_order_quantity_for_percentage(25.0, 1_000.0, false, None, 4),
        "0"
    );
}

#[test]
fn toggled_quick_order_quantity_converts_when_reference_price_is_available() {
    assert_eq!(
        toggled_quick_order_quantity_text("2.5", true, Some(100.0), 4),
        "250.00"
    );
    assert_eq!(
        toggled_quick_order_quantity_text("250", false, Some(100.0), 4),
        "2.5000"
    );
    assert_eq!(
        toggled_quick_order_quantity_text("1,234,500.00", false, Some(100.0), 2),
        "12,345.00"
    );
}

#[test]
fn quick_order_max_notional_uses_one_x_when_leverage_is_only_symbol_limit() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.account_data = Some(account_data_without_positions());

    assert_eq!(terminal.quick_order_max_notional("BTC"), Some(1_000.0));
}
