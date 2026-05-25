use super::*;

#[test]
fn position_and_order_overlay_render_guard_follows_privacy_flag() {
    let mut chart = CandlestickChart::new(1);
    assert!(chart.should_draw_position_and_order_overlays());

    chart.hide_positions_and_orders = true;
    assert!(!chart.should_draw_position_and_order_overlays());
}
