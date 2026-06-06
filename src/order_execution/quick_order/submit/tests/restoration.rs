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
