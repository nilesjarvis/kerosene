use super::*;

#[test]
fn handle_submit_quick_order_restores_form_when_symbol_metadata_is_missing() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "MISSING");

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "Symbol 'MISSING' not found");

    let instance = chart_instance_or_panic(&terminal, chart_id);
    let form = quick_order_or_panic(instance);
    assert!(instance.chart.quick_order_open);
    assert_eq!(instance.chart.quick_order_limit_price, Some(100.0));
    assert_eq!(form.quantity, "1.25");
    assert!(!form.quantity_is_usd);
    assert!(form.is_limit);
}

#[test]
fn handle_submit_quick_order_restores_form_when_shared_preflight_rejects_quantity() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    let instance = terminal
        .charts
        .get_mut(&chart_id)
        .expect("chart should exist");
    let form = instance
        .quick_order
        .as_mut()
        .expect("quick order should exist");
    form.quantity = "0".to_string();

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "Invalid quantity for asset precision");

    let instance = chart_instance_or_panic(&terminal, chart_id);
    let form = quick_order_or_panic(instance);
    assert!(instance.chart.quick_order_open);
    assert_eq!(form.quantity, "0");
}
