use super::*;
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::chart_state::{ChartId, ChartInstance};
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "1.25".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 400.0,
        chart_h: 300.0,
    }
}

fn terminal_with_quick_order(chart_id: ChartId, chart_symbol: &str) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.exchange_symbols.clear();

    let mut instance = ChartInstance::new(chart_id, chart_symbol.to_string(), Timeframe::H1);
    instance.set_quick_order(quick_order_form());
    terminal.charts.insert(chart_id, instance);
    terminal
}

#[test]
fn handle_submit_quick_order_restores_form_when_symbol_metadata_is_missing() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "MISSING");

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = terminal.order_status.as_ref().expect("status");
    assert!(*is_error);
    assert_eq!(message, "Symbol 'MISSING' not found");

    let instance = terminal.charts.get(&chart_id).expect("chart instance");
    let form = instance.quick_order.as_ref().expect("quick order restored");
    assert!(instance.chart.quick_order_open);
    assert_eq!(instance.chart.quick_order_limit_price, Some(100.0));
    assert_eq!(form.quantity, "1.25");
    assert!(!form.quantity_is_usd);
    assert!(form.is_limit);
}

#[test]
fn quick_order_size_wire_converts_usd_notional_to_coin_size() {
    assert_eq!(
        quick_order_size_wire("250", true, 100.0, 5),
        Some("2.5".into())
    );
    assert_eq!(
        quick_order_size_wire("2.5", false, 100.0, 5),
        Some("2.5".into())
    );
}

#[test]
fn quick_order_size_wire_quantizes_to_asset_precision() {
    assert_eq!(
        quick_order_size_wire("10", true, 30_000.0, 5),
        Some("0.00033".into())
    );
    assert_eq!(
        quick_order_size_wire("1.239", false, 100.0, 2),
        Some("1.23".into())
    );
}

#[test]
fn quick_order_size_wire_rejects_invalid_reference_for_usd() {
    assert_eq!(quick_order_size_wire("250", true, 0.0, 5), None);
    assert_eq!(quick_order_size_wire("250", true, f64::NAN, 5), None);
    assert_eq!(quick_order_size_wire("0", false, 100.0, 5), None);
    assert_eq!(quick_order_size_wire("-1", false, 100.0, 5), None);
    assert_eq!(quick_order_size_wire("NaN", false, 100.0, 5), None);
    assert_eq!(
        quick_order_size_wire("0.0000000000001", false, 100.0, 8),
        None
    );
    assert_eq!(quick_order_size_wire("10", true, 30_000.0, 2), None);
}

#[test]
fn quick_order_limit_price_wire_rejects_invalid_or_zero_rounded_prices() {
    assert_eq!(quick_order_limit_price_wire(f64::NAN, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(f64::INFINITY, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(0.0, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(-1.0, 2, false), None);
    assert_eq!(quick_order_limit_price_wire(0.0000001, 2, false), None);
}

#[test]
fn quick_order_limit_price_wire_returns_rounded_wire_price() {
    assert_eq!(
        quick_order_limit_price_wire(123.456789, 2, false),
        Some((123.46, "123.46".into()))
    );
}

#[test]
fn quick_order_market_price_wire_rejects_invalid_or_zero_prices() {
    assert_eq!(
        quick_order_market_price_wire(f64::NAN, true, 0.05, 2, false),
        None
    );
    assert_eq!(
        quick_order_market_price_wire(f64::INFINITY, true, 0.05, 2, false),
        None
    );
    assert_eq!(
        quick_order_market_price_wire(0.0000001, true, 0.05, 2, false),
        None
    );
}

#[test]
fn quick_order_market_price_wire_applies_slippage_and_rounding() {
    assert_eq!(
        quick_order_market_price_wire(100.0, true, 0.05, 2, false),
        Some((105.0, "105".into()))
    );
    assert_eq!(
        quick_order_market_price_wire(100.0, false, 0.05, 2, false),
        Some((95.0, "95".into()))
    );
}
