use super::*;

#[test]
fn active_order_line_phase_only_advances_when_orders_exist() {
    let mut instance = instance();

    instance.advance_order_line_animation();
    assert_eq!(instance.chart.order_line_phase, 0.0);

    instance.chart.active_orders.push(moving_order_overlay());
    instance.advance_order_line_animation();

    assert!(instance.chart.order_line_phase > 0.0);
}
