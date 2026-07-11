use super::{
    CHART_H, CHART_W, CandlestickChart, Message, Point, SURFACE_H, action_or_panic, btc_buy_order,
    chart_with_input_candles, message_or_panic, pending_btc_buy_order,
};
use crate::chart::ChartState;
use crate::chart::EarningsMarker;
use crate::chart::fisheye::ChartFisheye;
use crate::chart::interaction::{InteractionLayout, ProjectedCursor};
use crate::chart::order_labels;
use crate::chart::state::{DragKind, HudMarketSide, HudOrderKind};
use crate::config::ChartCrosshairStyle;
use crate::order_execution::{HudOrderSide, HudOrderType};

#[test]
fn quick_order_open_left_click_in_chart_area_closes_card_without_panning() {
    let mut chart = CandlestickChart::new(1);
    chart.quick_order_open = true;
    let mut state = ChartState::default();

    let action = action_or_panic(
        chart.handle_left_press(&mut state, Point::new(120.0, 80.0), CHART_W, CHART_H, 260.0),
        "left click should close an open quick-order card",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "close quick-order message") {
        Message::CloseQuickOrder(id) => assert_eq!(id, chart.id),
        other => panic!("expected CloseQuickOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn normal_left_click_marks_chart_focused_without_capturing_event() {
    let chart = CandlestickChart::new(7);
    let mut state = ChartState::default();

    let action = action_or_panic(
        chart.handle_left_press(&mut state, Point::new(120.0, 80.0), CHART_W, CHART_H, 260.0),
        "left click should publish chart focus",
    );
    let (message, _, status) = action.into_inner();

    assert!(matches!(message, Some(Message::ChartFocused(7))));
    assert_eq!(status, iced::event::Status::Ignored);
    assert!(matches!(state.drag, Some(DragKind::PanX)));
}

#[test]
fn earnings_marker_left_click_opens_filing_without_panning() {
    let mut chart = chart_with_input_candles();
    chart.set_earnings_markers(vec![EarningsMarker {
        time_ms: 2_000,
        cik: 1_652_044,
        form: "8-K".to_string(),
        filing_date: "2026-04-29".to_string(),
        accession_number: "0001652044-26-000043".to_string(),
        primary_document: "goog-20260429.htm".to_string(),
        quarter_label: Some("Q1 2026".to_string()),
        filing_summary: None,
        filing_summary_status: None,
        filing_summary_loading: false,
    }]);
    let mut state = ChartState::default();
    let marker_x = chart
        .timestamp_to_x(2_000, &state, CHART_W)
        .expect("marker x");
    let price_h = CHART_H * (1.0 - crate::chart::VOLUME_REGION_RATIO);
    let marker_y = price_h - 5.0;

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(marker_x, marker_y),
            CHART_W,
            CHART_H,
            260.0,
        ),
        "left click on earnings marker should open filing",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "open earnings filing message") {
        Message::OpenChartEarningsFiling(id, surface_id, time_ms) => {
            assert_eq!(id, chart.id);
            assert_eq!(surface_id, chart.surface_id);
            assert_eq!(time_ms, 2_000);
        }
        other => panic!("expected OpenChartEarningsFiling, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn session_divider_left_click_starts_resize_drag() {
    let mut chart = CandlestickChart::new(7);
    chart.macro_indicators.show_session_indicator = true;
    chart.set_session_panel_height(72.0);
    let mut state = ChartState::default();
    let (chart_h, funding_panel_h, session_panel_h) = chart.chart_area_heights(SURFACE_H);
    let divider_y = chart_h + funding_panel_h;

    let action = action_or_panic(
        chart.handle_left_press_at(
            &mut state,
            ProjectedCursor::identity(Point::new(120.0, divider_y + 1.0)),
            ChartFisheye::disabled(),
            InteractionLayout {
                chart_w: CHART_W,
                chart_h,
                funding_panel_h,
                session_panel_h,
            },
            SURFACE_H,
        ),
        "session divider left click should start resize drag",
    );
    let (message, _, status) = action.into_inner();

    assert!(message.is_none());
    assert_eq!(status, iced::event::Status::Captured);
    assert!(matches!(state.drag, Some(DragKind::ResizeSessionPanel)));
    assert_eq!(state.drag_start_session_panel_height, 72.0);
}

#[test]
fn pending_order_line_left_click_does_not_start_order_drag() {
    let mut chart = chart_with_input_candles();
    chart.active_orders.push(pending_btc_buy_order(42));
    let mut state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
        .expect("visible price params");
    let order_y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(120.0, order_y),
            CHART_W,
            CHART_H,
            260.0,
        ),
        "left click should focus chart instead of dragging a pending order",
    );
    let (message, _, _) = action.into_inner();

    assert!(matches!(message, Some(Message::ChartFocused(1))));
    assert!(matches!(state.drag, Some(DragKind::PanX)));
}

#[test]
fn fisheye_left_click_on_order_cancel_button_cancels_order() {
    let mut chart = chart_with_input_candles();
    chart.active_orders.push(btc_buy_order(42));
    chart.set_fisheye(true, 1.0);
    let mut state = ChartState::default();
    let fisheye = ChartFisheye::new(true, chart.fisheye_strength, CHART_W, CHART_H);
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
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
    let (cancel_x, cancel_end_x) = order_labels::order_cancel_x_range(&chart.active_orders[0]);
    let visual_label_y = fisheye
        .project(Point::new(order_labels::ORDER_LABEL_X, label.label_y))
        .y;
    let visual_click = Point::new((cancel_x + cancel_end_x) * 0.5, visual_label_y);
    let source_click = fisheye.unproject(visual_click);

    let action = action_or_panic(
        chart.handle_left_press_at(
            &mut state,
            ProjectedCursor {
                source: source_click,
                visual: visual_click,
            },
            fisheye,
            InteractionLayout::without_funding(CHART_W, CHART_H),
            260.0,
        ),
        "fisheye visual click on order cancel should publish cancel message",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "cancel order message") {
        Message::CancelOrder { coin, oid } => {
            assert_eq!(coin, "BTC");
            assert_eq!(oid.into_u64(), 42);
        }
        other => panic!("expected CancelOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn hud_left_click_submits_hud_order_without_starting_pan() {
    let mut chart = chart_with_input_candles();
    chart.set_symbol_key("BTC".to_string());
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 0);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Market,
        hud_market_side: HudMarketSide::Short,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    let action = action_or_panic(
        chart.handle_left_press(&mut state, Point::new(120.0, 80.0), CHART_W, CHART_H, 260.0),
        "HUD left click should submit an order request",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "submit HUD order message") {
        Message::SubmitHudOrder(request) => {
            assert_eq!(request.chart_id, chart.id);
            assert_eq!(request.surface_id, chart.surface_id);
            assert_eq!(request.symbol_key, "BTC");
            assert_eq!(request.quantity, "2.5");
            assert_eq!(request.order_type, HudOrderType::Market);
            assert_eq!(request.market_side, HudOrderSide::Short);
            assert_eq!(request.limit_side, None);
            assert_eq!(request.click_x, 120.0);
            assert_eq!(request.click_y, 80.0);
            assert_eq!(request.chart_w, CHART_W);
            assert_eq!(request.chart_h, CHART_H);
            assert!(request.price.is_finite() && request.price > 0.0);
        }
        other => panic!("expected SubmitHudOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn hud_armed_click_inside_weapon_station_is_swallowed() {
    let mut chart = chart_with_input_candles();
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 0);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Market,
        hud_market_side: HudMarketSide::Short,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    // Bottom-right corner of the plot, inside the weapon station bounds.
    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(300.0, 170.0),
            CHART_W,
            CHART_H,
            260.0,
        ),
        "armed click on the weapon station should be captured",
    );
    let (message, _, status) = action.into_inner();

    assert!(
        message.is_none(),
        "station deadzone must not fire an order, got {message:?}"
    );
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none(), "deadzone click must not start a pan");
}

#[test]
fn hud_station_deadzone_hit_tests_the_physical_cursor_not_the_unprojected_point() {
    let mut chart = chart_with_input_candles();
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 0);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Market,
        hud_market_side: HudMarketSide::Short,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };
    let layout = InteractionLayout::without_funding(CHART_W, CHART_H);

    // Fisheye pulls the unprojected point off the station; the click is
    // physically on the drawn station and must be swallowed regardless.
    let on_station = ProjectedCursor {
        source: Point::new(240.0, 120.0),
        visual: Point::new(300.0, 170.0),
    };
    let action = action_or_panic(
        chart.handle_left_press_at(
            &mut state,
            on_station,
            ChartFisheye::disabled(),
            layout,
            260.0,
        ),
        "armed click physically on the station should be captured",
    );
    let (message, _, _) = action.into_inner();
    assert!(
        message.is_none(),
        "click on the drawn station must not fire, got {message:?}"
    );

    // Mirror case: physically outside the station fires even though the
    // unprojected point lands inside the station bounds.
    let off_station = ProjectedCursor {
        source: Point::new(300.0, 170.0),
        visual: Point::new(120.0, 80.0),
    };
    let action = action_or_panic(
        chart.handle_left_press_at(
            &mut state,
            off_station,
            ChartFisheye::disabled(),
            layout,
            260.0,
        ),
        "armed click physically on the plot should fire",
    );
    let (message, _, _) = action.into_inner();
    match message_or_panic(message, "submit HUD order message") {
        Message::SubmitHudOrder(_) => {}
        other => panic!("expected SubmitHudOrder, got {other:?}"),
    }
}

#[test]
fn racing_hud_left_click_submits_hud_order_without_starting_pan() {
    let mut chart = chart_with_input_candles();
    chart.set_crosshair_style(ChartCrosshairStyle::RacingHud);
    chart.set_hud_armed_at(true, 0);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Market,
        hud_market_side: HudMarketSide::Short,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    let action = action_or_panic(
        chart.handle_left_press(&mut state, Point::new(120.0, 80.0), CHART_W, CHART_H, 260.0),
        "Racing HUD left click should submit an order request",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "submit HUD order message") {
        Message::SubmitHudOrder(request) => {
            assert_eq!(request.chart_id, chart.id);
            assert_eq!(request.surface_id, chart.surface_id);
            assert_eq!(request.quantity, "2.5");
            assert_eq!(request.order_type, HudOrderType::Market);
            assert_eq!(request.market_side, HudOrderSide::Short);
            assert_eq!(request.limit_side, None);
        }
        other => panic!("expected SubmitHudOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn hud_limit_left_click_captures_click_time_side() {
    let mut chart = chart_with_input_candles();
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 0);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Limit,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
        .expect("visible price params");
    let click_y = chart.price_to_y_with(100.0, price_hi, price_range, price_h);

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(120.0, click_y),
            CHART_W,
            CHART_H,
            260.0,
        ),
        "HUD limit left click should submit an order request",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "submit HUD order message") {
        Message::SubmitHudOrder(request) => {
            assert_eq!(request.order_type, HudOrderType::Limit);
            assert_eq!(request.limit_side, Some(HudOrderSide::Long));
            assert!(request.price <= 110.0);
        }
        other => panic!("expected SubmitHudOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn hud_limit_left_click_uses_live_reference_for_click_time_side() {
    let mut chart = chart_with_input_candles();
    chart.set_market_reference_price(Some(90.0));
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 0);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Limit,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
        .expect("visible price params");
    let click_y = chart.price_to_y_with(100.0, price_hi, price_range, price_h);

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(120.0, click_y),
            CHART_W,
            CHART_H,
            260.0,
        ),
        "HUD limit left click should submit an order request",
    );
    let (message, _, status) = action.into_inner();

    match message_or_panic(message, "submit HUD order message") {
        Message::SubmitHudOrder(request) => {
            assert_eq!(request.order_type, HudOrderType::Limit);
            assert_eq!(request.limit_side, Some(HudOrderSide::Short));
            assert!(request.price > 90.0);
        }
        other => panic!("expected SubmitHudOrder, got {other:?}"),
    }
    assert_eq!(status, iced::event::Status::Captured);
    assert!(state.drag.is_none());
}

#[test]
fn hud_left_click_is_pan_when_not_armed() {
    let mut chart = chart_with_input_candles();
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    let mut state = ChartState {
        hud_order_kind: HudOrderKind::Market,
        hud_market_side: HudMarketSide::Short,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    let action = action_or_panic(
        chart.handle_left_press(&mut state, Point::new(120.0, 80.0), CHART_W, CHART_H, 260.0),
        "unarmed HUD left click should fall through to chart pan",
    );
    let (message, _, status) = action.into_inner();

    assert!(matches!(message, Some(Message::ChartFocused(1))));
    assert_eq!(status, iced::event::Status::Ignored);
    assert!(matches!(state.drag, Some(DragKind::PanX)));
}
