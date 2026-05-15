use super::*;
use crate::annotations::{Annotation, AnnotationKind, DEFAULT_LEVEL_COLOR};
use crate::api::Candle;
use crate::chart::state::DragKind;
use crate::hydromancer_api::FundingRatePoint;
use crate::message::Message;
use iced::{Point, Rectangle};

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 10.0,
    }
}

#[test]
fn merge_candles_replaces_overlaps_and_keeps_sorted_order() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0), candle_at(2_000, 20.0)]);

    chart.merge_candles(vec![candle_at(3_000, 30.0), candle_at(2_000, 22.0)]);

    assert_eq!(
        chart
            .candles
            .iter()
            .map(|candle| (candle.open_time, candle.close))
            .collect::<Vec<_>>(),
        vec![(1_000, 10.0), (2_000, 22.0), (3_000, 30.0)]
    );
    assert!(matches!(chart.status, ChartStatus::Loaded));
}

#[test]
fn realtime_candle_update_rejects_malformed_candles() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0)]);
    let mut invalid = candle_at(1_000, 20.0);
    invalid.close = f64::NAN;

    chart.push_candle(invalid);

    assert_eq!(chart.candles.len(), 1);
    assert_eq!(chart.candles[0].close, 10.0);
}

#[test]
fn merge_funding_history_updates_overlaps_and_keeps_sorted_order() {
    let mut chart = CandlestickChart::new(1);
    chart.set_funding_history(vec![
        FundingRatePoint {
            time_ms: 1_000,
            rate: 0.01,
        },
        FundingRatePoint {
            time_ms: 2_000,
            rate: 0.02,
        },
    ]);

    chart.merge_funding_history(vec![
        FundingRatePoint {
            time_ms: 2_000,
            rate: -0.03,
        },
        FundingRatePoint {
            time_ms: 3_000,
            rate: 0.04,
        },
    ]);

    assert_eq!(
        chart
            .funding_rates
            .iter()
            .map(|point| (point.time_ms, point.rate))
            .collect::<Vec<_>>(),
        vec![(1_000, 0.01), (2_000, -0.03), (3_000, 0.04)]
    );
}

#[test]
fn empty_incremental_funding_update_preserves_existing_points() {
    let mut chart = CandlestickChart::new(1);
    chart.set_funding_history(vec![FundingRatePoint {
        time_ms: 1_000,
        rate: 0.01,
    }]);

    chart.merge_funding_history(Vec::new());

    assert_eq!(chart.funding_rates.len(), 1);
    assert_eq!(chart.funding_rates[0].rate, 0.01);
}

#[test]
fn quick_order_open_left_click_in_chart_area_closes_card_without_panning() {
    let mut chart = CandlestickChart::new(1);
    chart.quick_order_open = true;
    let mut state = ChartState::default();

    let action = chart
        .handle_left_press(&mut state, Point::new(120.0, 80.0), 400.0, 240.0, 260.0)
        .expect("left click should close an open quick-order card");
    let (message, _, status) = action.into_inner();

    match message.expect("close quick-order message") {
        Message::CloseQuickOrder(id) => assert_eq!(id, chart.id),
        other => panic!("expected CloseQuickOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn quick_order_open_right_click_in_chart_area_publishes_replacement_open_message() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    chart.quick_order_open = true;
    let mut state = ChartState::default();

    let action = chart
        .handle_right_press(
            &mut state,
            Rectangle::new(Point::ORIGIN, iced::Size::new(420.0, 260.0)),
            Point::new(120.0, 80.0),
            400.0,
            240.0,
        )
        .expect("right click should publish replacement quick-order open message");
    let (message, _, status) = action.into_inner();

    match message.expect("open quick-order message") {
        Message::OpenQuickOrder(id, price, click_x, click_y, chart_w, chart_h) => {
            assert_eq!(id, chart.id);
            assert!(price.is_finite() && price > 0.0);
            assert_eq!(click_x, 120.0);
            assert_eq!(click_y, 80.0);
            assert_eq!(chart_w, 400.0);
            assert_eq!(chart_h, 240.0);
        }
        other => panic!("expected OpenQuickOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
}

#[test]
fn quick_order_open_right_click_on_order_line_still_replaces_card_not_cancel_order() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    chart.quick_order_open = true;
    chart.active_orders.push(OrderOverlay {
        coin: "BTC".to_string(),
        limit_px: 105.0,
        sz: 1.0,
        is_buy: true,
        oid: 42,
        is_moving: false,
    });
    let mut state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, 400.0, 240.0)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);

    let action = chart
        .handle_right_press(
            &mut state,
            Rectangle::new(Point::ORIGIN, iced::Size::new(420.0, 260.0)),
            Point::new(120.0, order_y),
            400.0,
            240.0,
        )
        .expect("right click should replace quick-order even on an order line");
    let (message, _, status) = action.into_inner();

    match message.expect("open quick-order message") {
        Message::OpenQuickOrder(id, price, click_x, click_y, chart_w, chart_h) => {
            assert_eq!(id, chart.id);
            assert!(price.is_finite() && price > 0.0);
            assert_eq!(click_x, 120.0);
            assert_eq!(click_y, order_y);
            assert_eq!(chart_w, 400.0);
            assert_eq!(chart_h, 240.0);
        }
        other => panic!("expected OpenQuickOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
}

#[test]
fn quick_order_open_right_click_replaces_even_if_range_anchor_is_set() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    chart.quick_order_open = true;
    let mut state = ChartState {
        range_anchor_price: Some(101.0),
        ..ChartState::default()
    };

    let action = chart
        .handle_right_press(
            &mut state,
            Rectangle::new(Point::ORIGIN, iced::Size::new(420.0, 260.0)),
            Point::new(120.0, 80.0),
            400.0,
            240.0,
        )
        .expect("right click should replace quick-order instead of clearing range anchor");
    let (message, _, _) = action.into_inner();

    assert!(matches!(message, Some(Message::OpenQuickOrder(..))));
    assert_eq!(state.range_anchor_price, Some(101.0));
}

#[test]
fn reset_view_state_restores_default_positioning() {
    let mut state = ChartState {
        scroll_offset: 42.0,
        candle_width: 18.0,
        y_auto: false,
        y_offset: 123.0,
        y_scale: 4.0,
        drag: Some(DragKind::PanX),
        drag_start: Some(Point::new(1.0, 2.0)),
        drag_start_scroll: 7.0,
        drag_start_y_offset: 9.0,
        drag_order_new_price: Some(100.0),
        hover_order_oid: Some(9),
        pending_anchor: Some((1_000, 10.0)),
        range_anchor_price: Some(11.0),
        ..ChartState::default()
    };

    state.reset_view(5);

    assert_eq!(state.scroll_offset, 0.0);
    assert_eq!(state.candle_width, DEFAULT_CANDLE_WIDTH);
    assert!(state.y_auto);
    assert_eq!(state.y_offset, 0.0);
    assert_eq!(state.y_scale, 1.0);
    assert!(state.drag.is_none());
    assert!(state.drag_start.is_none());
    assert!(state.drag_order_new_price.is_none());
    assert!(state.hover_order_oid.is_none());
    assert!(state.pending_anchor.is_none());
    assert!(state.range_anchor_price.is_none());
    assert_eq!(state.reset_epoch_seen, 5);
}

#[test]
fn reset_request_preserves_chart_content() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0), candle_at(2_000, 20.0)]);
    chart.annotations.push(Annotation {
        id: 1,
        kind: AnnotationKind::HorizontalLevel { price: 12.0 },
        color: DEFAULT_LEVEL_COLOR,
    });

    chart.request_view_reset();

    assert_eq!(chart.reset_epoch, 1);
    assert_eq!(chart.candles.len(), 2);
    assert_eq!(chart.annotations.len(), 1);
    assert!(matches!(chart.status, ChartStatus::Loaded));
}

#[test]
fn visible_range_clamps_past_overscroll_to_first_candle() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![
        candle_at(1_000, 10.0),
        candle_at(2_000, 20.0),
        candle_at(3_000, 30.0),
    ]);
    let state = ChartState {
        scroll_offset: 100.0,
        ..ChartState::default()
    };

    let range = chart
        .visible_candle_range(&state, 400.0)
        .expect("range should clamp into candle data");
    let first_x = chart
        .timestamp_to_x(1_000, &state, 400.0)
        .expect("first candle should have an x coordinate");

    assert_eq!(range.first, 0);
    assert_eq!(range.last, 0);
    assert!((0.0..=400.0).contains(&first_x));
}

#[test]
fn visible_range_clamps_future_overscroll_to_latest_candles() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![
        candle_at(1_000, 10.0),
        candle_at(2_000, 20.0),
        candle_at(3_000, 30.0),
    ]);
    let state = ChartState {
        scroll_offset: -1_000.0,
        ..ChartState::default()
    };

    let range = chart
        .visible_candle_range(&state, 400.0)
        .expect("range should clamp into candle data");

    assert!(range.first <= range.last);
    assert_eq!(range.last, 2);
}

#[test]
fn one_candle_visible_range_is_stable() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0)]);
    let state = ChartState {
        scroll_offset: 100.0,
        ..ChartState::default()
    };

    let range = chart
        .visible_candle_range(&state, 400.0)
        .expect("single candle should still be visible");

    assert_eq!(range.first, 0);
    assert_eq!(range.last, 0);
    assert!(chart.visible_price_params(&state, 400.0, 300.0).is_some());
}
