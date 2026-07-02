use super::quick_order_fee_quantity;
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

fn quick_order_form(quantity: &str, quantity_is_usd: bool) -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: quantity.to_string(),
        quantity_is_usd,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 0.0,
        click_y: 0.0,
        chart_w: 0.0,
        chart_h: 0.0,
    }
}

fn exchange_symbol(
    key: &str,
    ticker: &str,
    sz_decimals: u32,
    market_type: MarketType,
) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

/// Terminal whose globally active symbol is a BTC perp while chart 7 hosts a
/// spot quick-order HUD, mirroring a detached spot chart window.
fn terminal_with_spot_chart_and_perp_active_symbol(form: QuickOrderForm) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![
        exchange_symbol("BTC", "BTC", 5, MarketType::Perp),
        exchange_symbol("@107", "HYPE", 2, MarketType::Spot),
    ];
    let now_ms = TradingTerminal::now_ms();
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), now_ms);
    terminal.all_mids.insert("@107".to_string(), 4.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@107".to_string(), now_ms);
    let mut instance = ChartInstance::new(7, "@107".to_string(), Timeframe::H1);
    instance.set_quick_order(form);
    terminal.charts.insert(7, instance);
    terminal
}

#[test]
fn quick_order_usd_fee_quantity_converts_notional_to_base_size() {
    let form = quick_order_form("250", true);
    assert_eq!(quick_order_fee_quantity(&form, 100.0, 5), Some(2.5));
}

#[test]
fn quick_order_coin_fee_quantity_uses_asset_precision() {
    let form = quick_order_form("1.239", false);
    assert_eq!(quick_order_fee_quantity(&form, 100.0, 2), Some(1.23));
}

#[test]
fn quick_order_fee_inputs_use_chart_symbol_not_active_symbol_for_limit_orders() {
    let mut form = quick_order_form("10.239", false);
    form.price = 4.0;
    let terminal = terminal_with_spot_chart_and_perp_active_symbol(form.clone());

    let (price, quantity, is_spot) = terminal
        .quick_order_fee_inputs(7, &form)
        .expect("fee inputs for the spot chart HUD");

    assert_eq!(price, 4.0);
    // Quantized with the spot pair's 2 size decimals, not BTC's 5.
    assert_eq!(quantity, 10.23);
    assert!(is_spot);
}

#[test]
fn quick_order_fee_inputs_use_chart_symbol_mid_for_market_orders() {
    let mut form = quick_order_form("10", false);
    form.is_limit = false;
    form.price = 0.0;
    let terminal = terminal_with_spot_chart_and_perp_active_symbol(form.clone());

    let (price, quantity, is_spot) = terminal
        .quick_order_fee_inputs(7, &form)
        .expect("fee inputs for the spot chart HUD");

    // The HYPE/USDC spot mid, not the active BTC perp mid.
    assert_eq!(price, 4.0);
    assert_eq!(quantity, 10.0);
    assert!(is_spot);
}

#[test]
fn quick_order_fee_inputs_without_chart_instance_are_unavailable() {
    let form = quick_order_form("10", false);
    let terminal = terminal_with_spot_chart_and_perp_active_symbol(form.clone());

    assert!(terminal.quick_order_fee_inputs(99, &form).is_none());
}
