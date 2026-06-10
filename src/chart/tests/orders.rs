use super::candle_at;
use crate::chart::order_labels;
use crate::chart::{
    CandlestickChart, ChartState, OrderOverlay, OrderOverlayPendingState, PositionOverlay,
};
use iced::Point;

fn chart_with_order_candles() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    chart
}

fn btc_buy_order(oid: u64, sz: f64) -> OrderOverlay {
    OrderOverlay {
        coin: "BTC".to_string(),
        limit_px: 105.0,
        sz,
        is_buy: true,
        oid,
        is_moving: false,
        pending_state: None,
    }
}

fn btc_position(entry_px: f64) -> PositionOverlay {
    PositionOverlay {
        entry_px,
        szi: 1.0,
        liquidation_px: None,
    }
}

#[test]
fn order_hit_testing_follows_stacked_left_labels() {
    let mut chart = chart_with_order_candles();
    chart.active_orders.push(btc_buy_order(41, 1.0));
    chart.active_orders.push(btc_buy_order(42, 2.0));
    let state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, 400.0, 240.0)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);
    let label_positions = order_labels::order_label_position_slots(
        order_labels::stack_order_label_positions_avoiding(
            vec![
                order_labels::OrderLabelAnchor {
                    order_index: 0,
                    order_y,
                    is_buy: true,
                },
                order_labels::OrderLabelAnchor {
                    order_index: 1,
                    order_y,
                    is_buy: true,
                },
            ],
            price_h,
            &[],
        ),
        chart.active_orders.len(),
    );
    let second_label = order_labels::order_label_position(&label_positions, 1)
        .expect("second label should be laid out");

    let hit = chart
        .hit_test_order_line(&state, Point::new(6.0, second_label.label_y), 400.0, 240.0)
        .expect("stacked label should be hittable");

    assert_eq!(hit.order.oid, 42);
    assert!(hit.is_label_hit());
}

#[test]
fn order_label_hit_testing_avoids_active_position_label() {
    let mut chart = chart_with_order_candles();
    chart.active_position = Some(btc_position(105.0));
    chart.active_orders.push(btc_buy_order(42, 1.0));
    let state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, 400.0, 240.0)
        .expect("visible price params");
    let price_to_y = |price| chart.price_to_y_with(price, price_hi, price_range, price_h);
    let order_y = price_to_y(105.0);
    let label_positions = order_labels::order_label_position_slots(
        order_labels::stack_order_label_positions_avoiding(
            vec![order_labels::OrderLabelAnchor {
                order_index: 0,
                order_y,
                is_buy: true,
            }],
            price_h,
            &chart.order_label_reserved_ranges(price_h, &price_to_y),
        ),
        chart.active_orders.len(),
    );
    let label = order_labels::order_label_position(&label_positions, 0)
        .expect("order label should be laid out");

    assert!(label.label_y > order_y);

    let hit = chart
        .hit_test_order_line(&state, Point::new(6.0, label.label_y), 400.0, 240.0)
        .expect("shifted order label should be hittable");

    assert_eq!(hit.order.oid, 42);
    assert!(hit.is_label_hit());
}

#[test]
fn order_cancel_hit_testing_uses_expanded_circle_target() {
    let mut chart = chart_with_order_candles();
    chart.active_orders.push(btc_buy_order(42, 1.0));
    let state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, 400.0, 240.0)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);
    let label_positions = order_labels::order_label_position_slots(
        order_labels::stack_order_label_positions_avoiding(
            vec![order_labels::OrderLabelAnchor {
                order_index: 0,
                order_y,
                is_buy: true,
            }],
            price_h,
            &[],
        ),
        chart.active_orders.len(),
    );
    let label = order_labels::order_label_position(&label_positions, 0)
        .expect("order label should be laid out");
    let (_, cancel_end_x) = order_labels::order_cancel_x_range(&chart.active_orders[0]);

    let pos = Point::new(cancel_end_x + 4.0, label.label_y);

    assert_eq!(
        chart.hit_test_order_cancel(&state, pos, 400.0, 240.0),
        Some(42)
    );
}

#[test]
fn order_hit_testing_matches_drawn_stack_when_pending_overlay_present() {
    let mut chart = chart_with_order_candles();
    chart.active_orders.push(btc_buy_order(41, 1.0));
    let mut pending = btc_buy_order(999, 1.0);
    pending.pending_state = Some(OrderOverlayPendingState::Placing);
    chart.active_orders.push(pending);
    chart.active_orders.push(btc_buy_order(42, 2.0));
    let state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, 400.0, 240.0)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);

    // Drawing stacks labels over ALL overlays, including pending ones; the
    // clickable positions must use the same geometry or a click on a drawn
    // cancel X lands on a different order.
    let label_positions = order_labels::order_label_position_slots(
        order_labels::stack_order_label_positions_avoiding(
            (0..chart.active_orders.len())
                .map(|order_index| order_labels::OrderLabelAnchor {
                    order_index,
                    order_y,
                    is_buy: true,
                })
                .collect(),
            price_h,
            &[],
        ),
        chart.active_orders.len(),
    );
    let drawn_label = order_labels::order_label_position(&label_positions, 2)
        .expect("real order label should be laid out");

    let hit = chart
        .hit_test_order_line(&state, Point::new(6.0, drawn_label.label_y), 400.0, 240.0)
        .expect("drawn label position should be hittable");
    assert_eq!(hit.order.oid, 42);
    assert!(hit.is_label_hit());

    let (_, cancel_end_x) = order_labels::order_cancel_x_range(&chart.active_orders[2]);
    assert_eq!(
        chart.hit_test_order_cancel(
            &state,
            Point::new(cancel_end_x + 4.0, drawn_label.label_y),
            400.0,
            240.0
        ),
        Some(42)
    );
}

#[test]
fn order_cancel_hover_animation_eases_toward_target() {
    let mut chart = chart_with_order_candles();

    chart.set_order_cancel_hover(Some(42));
    chart.advance_order_cancel_hover_animation();

    assert!(chart.order_cancel_hover_animation_active());
    assert!(chart.order_cancel_hover_progress() > 0.0);
    assert!(chart.order_cancel_hover_progress_for(42) > chart.order_cancel_hover_progress());

    chart.set_order_cancel_hover(None);
    for _ in 0..20 {
        chart.advance_order_cancel_hover_animation();
    }

    assert!(chart.order_cancel_hover_progress() < 0.05);
}

#[test]
fn order_hit_testing_ignores_invalid_resize_bounds() {
    let mut chart = chart_with_order_candles();
    chart.active_orders.push(btc_buy_order(42, 1.0));
    let state = ChartState::default();
    let pos = Point::new(10.0, 10.0);

    assert!(chart.hit_test_order_line(&state, pos, 0.0, 240.0).is_none());
    assert!(chart.hit_test_order_line(&state, pos, 400.0, 0.0).is_none());
    assert!(
        chart
            .hit_test_order_line(&state, pos, f32::NAN, 240.0)
            .is_none()
    );
    assert!(
        chart
            .hit_test_order_line(&state, Point::new(f32::NAN, 10.0), 400.0, 240.0)
            .is_none()
    );
}
