use super::{
    CHART_H, CHART_W, CandlestickChart, Message, Point, action_or_panic, message_or_panic,
};
use crate::chart::ChartState;
use crate::chart::state::DragKind;

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
