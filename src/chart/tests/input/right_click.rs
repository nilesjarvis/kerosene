use super::{
    CHART_H, CHART_W, Point, SURFACE_H, SURFACE_W, action_or_panic,
    assert_open_quick_order_message, btc_buy_order, chart_bounds, pending_btc_buy_order,
    quick_order_chart,
};
use crate::chart::ChartState;
use crate::message::Message;

#[test]
fn quick_order_open_right_click_in_chart_area_publishes_replacement_open_message() {
    let chart = quick_order_chart();
    let mut state = ChartState::default();
    let click = Point::new(120.0, 80.0);

    let action = action_or_panic(
        chart.handle_right_press(
            &mut state,
            chart_bounds(SURFACE_W, SURFACE_H),
            click,
            CHART_W,
            CHART_H,
        ),
        "right click should publish replacement quick-order open message",
    );
    let (message, _, status) = action.into_inner();

    assert_open_quick_order_message(message, &chart, click);
    assert_eq!(status, iced::event::Status::Captured);
}

#[test]
fn quick_order_open_right_click_on_order_line_still_replaces_card_not_cancel_order() {
    let mut chart = quick_order_chart();
    chart.active_orders.push(btc_buy_order(42));
    let mut state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);
    let click = Point::new(120.0, order_y);

    let action = action_or_panic(
        chart.handle_right_press(
            &mut state,
            chart_bounds(SURFACE_W, SURFACE_H),
            click,
            CHART_W,
            CHART_H,
        ),
        "right click should replace quick-order even on an order line",
    );
    let (message, _, status) = action.into_inner();

    assert_open_quick_order_message(message, &chart, click);
    assert_eq!(status, iced::event::Status::Captured);
}

#[test]
fn quick_order_open_right_click_replaces_even_if_range_anchor_is_set() {
    let chart = quick_order_chart();
    let mut state = ChartState {
        range_anchor_price: Some(101.0),
        ..ChartState::default()
    };

    let action = action_or_panic(
        chart.handle_right_press(
            &mut state,
            chart_bounds(SURFACE_W, SURFACE_H),
            Point::new(120.0, 80.0),
            CHART_W,
            CHART_H,
        ),
        "right click should replace quick-order instead of clearing range anchor",
    );
    let (message, _, _) = action.into_inner();

    assert!(matches!(message, Some(Message::OpenQuickOrder(..))));
    assert_eq!(state.range_anchor_price, Some(101.0));
}

#[test]
fn pending_order_line_right_click_opens_quick_order_instead_of_canceling() {
    let mut chart = super::chart_with_input_candles();
    chart.active_orders.push(pending_btc_buy_order(42));
    let mut state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);
    let click = Point::new(120.0, order_y);

    let action = action_or_panic(
        chart.handle_right_press(
            &mut state,
            chart_bounds(SURFACE_W, SURFACE_H),
            click,
            CHART_W,
            CHART_H,
        ),
        "right click should open quick-order instead of cancelling a pending order",
    );
    let (message, _, status) = action.into_inner();

    assert_open_quick_order_message(message, &chart, click);
    assert_eq!(status, iced::event::Status::Captured);
}
