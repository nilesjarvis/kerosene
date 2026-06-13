use super::*;

#[test]
fn removing_chase_order_resyncs_chart_order_overlays() {
    let mut terminal = chase_ready_terminal();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    chart_instance_mut(&mut terminal, 1)
        .chart
        .active_orders
        .push(OrderOverlay {
            coin: "BTC".to_string(),
            limit_px: 100.0,
            sz: 1.0,
            is_buy: true,
            oid: 42,
            is_moving: false,
            pending_state: None,
        });
    terminal.account_data_address = Some(TEST_ACCOUNT.to_string());
    terminal.account_data = Some(account_data_with_order(open_order(42)));

    let _task = terminal.handle_chase_resting_order("BTC".to_string(), 42);
    let chase_id = selected_chase_id(&terminal);
    terminal.account_data = None;

    terminal.remove_chase_order(chase_id);

    assert!(chart_instance(&terminal, 1).chart.active_orders.is_empty());
}
