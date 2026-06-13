use super::*;

#[test]
fn clear_detached_surface_state_removes_only_detached_quick_order_owner() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    let _task = terminal.open_detached_chart_window(chart_id);
    let (window_id, detached_chart_id) = first_detached_window(&terminal);
    let surface_id = ChartSurfaceId::Detached(window_id);
    let instance = chart_instance_mut(&mut terminal, detached_chart_id);

    instance.set_quick_order(QuickOrderForm {
        price: 100.0,
        quantity: "1".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    });
    terminal
        .chart_quick_order_surface
        .insert(detached_chart_id, surface_id);

    assert!(terminal.chart_surface_has_quick_order(detached_chart_id, surface_id));

    terminal.clear_chart_surface_state(detached_chart_id, surface_id);

    assert!(!terminal.chart_surface_has_quick_order(detached_chart_id, surface_id));
    assert!(
        !chart_instance(&terminal, detached_chart_id)
            .chart
            .quick_order_open
    );
}

#[test]
fn clear_chart_pending_request_state_prunes_closed_chart_from_shared_registries() {
    let chart_id = 7;
    let other_chart_id = 99;
    let mut terminal = terminal_with_chart(chart_id);
    seed_chart_pending_requests(&mut terminal, chart_id, other_chart_id);

    terminal.clear_chart_pending_request_state(chart_id);

    assert_chart_pending_requests_pruned(&terminal, other_chart_id);
}
