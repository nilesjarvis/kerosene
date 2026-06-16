use super::candle_at;
use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle, DEFAULT_LEVEL_COLOR};
use crate::chart::state::DragKind;
use crate::chart::{CandlestickChart, ChartState, ChartStatus, DEFAULT_CANDLE_WIDTH};
use iced::Point;

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
        draft_anchors: vec![(1_000, 10.0)],
        selected_annotation: Some(3),
        hud_follow_price: true,
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
    assert!(state.draft_anchors.is_empty());
    assert!(state.selected_annotation.is_none());
    assert!(!state.hud_follow_price);
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
        style: AnnotationStyle {
            color: DEFAULT_LEVEL_COLOR,
            ..AnnotationStyle::default()
        },
    });

    chart.request_view_reset();

    assert_eq!(chart.reset_epoch, 1);
    assert_eq!(chart.candles.len(), 2);
    assert_eq!(chart.annotations.len(), 1);
    assert!(matches!(chart.status, ChartStatus::Loaded));
}
