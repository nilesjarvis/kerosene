use super::{
    CHART_H, CHART_W, SURFACE_H, SURFACE_W, action_or_panic, chart_with_input_candles,
    message_or_panic,
};
use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle, DrawingTool, FibKind};
use crate::chart::state::DragKind;
use crate::chart::tests::chart_bounds;
use crate::chart::{CandlestickChart, ChartState};
use crate::message::Message;
use iced::event::Status;
use iced::{Point, keyboard};

fn level(id: u64, price: f64) -> Annotation {
    Annotation {
        id,
        kind: AnnotationKind::HorizontalLevel { price },
        style: AnnotationStyle::default(),
    }
}

#[test]
fn two_click_trend_line_commits_on_second_click() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::TrendLine);
    let mut state = ChartState::default();

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(100.0, 80.0),
            CHART_W,
            CHART_H,
            SURFACE_H,
        ),
        "first trend-line click",
    );
    let (message, _, status) = action.into_inner();
    assert!(message.is_none(), "first click must not commit");
    assert_eq!(status, Status::Captured);
    assert_eq!(state.draft_anchors.len(), 1);
    assert_eq!(state.draft_tool, Some(DrawingTool::TrendLine));

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(200.0, 120.0),
            CHART_W,
            CHART_H,
            SURFACE_H,
        ),
        "second trend-line click",
    );
    let (message, _, status) = action.into_inner();
    match message_or_panic(message, "add annotation") {
        Message::AddAnnotation(id, ann) => {
            assert_eq!(id, chart.id);
            assert!(matches!(ann.kind, AnnotationKind::TrendLine { .. }));
        }
        other => panic!("expected AddAnnotation, got {other:?}"),
    }
    assert_eq!(status, Status::Captured);
    assert!(state.draft_anchors.is_empty(), "draft clears after commit");
    assert_eq!(state.draft_tool, None);
}

#[test]
fn three_click_fib_extension_commits_on_third_click() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::FibExtension);
    let mut state = ChartState::default();

    for (i, x) in [80.0, 160.0].into_iter().enumerate() {
        let action = action_or_panic(
            chart.handle_left_press(
                &mut state,
                Point::new(x, 100.0),
                CHART_W,
                CHART_H,
                SURFACE_H,
            ),
            "fib extension click",
        );
        let (message, _, _) = action.into_inner();
        assert!(message.is_none(), "click {i} must not commit");
        assert_eq!(state.draft_anchors.len(), i + 1);
    }

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(240.0, 140.0),
            CHART_W,
            CHART_H,
            SURFACE_H,
        ),
        "third fib extension click",
    );
    let (message, _, _) = action.into_inner();
    match message_or_panic(message, "add fib") {
        Message::AddAnnotation(_, ann) => assert!(matches!(
            ann.kind,
            AnnotationKind::Fib {
                kind: FibKind::Extension,
                ref points,
            } if points.len() == 3
        )),
        other => panic!("expected AddAnnotation, got {other:?}"),
    }
    assert!(state.draft_anchors.is_empty());
}

#[test]
fn escape_cancels_in_progress_draft_without_committing() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::TrendLine);
    let mut state = ChartState::default();

    let _ = chart.handle_left_press(
        &mut state,
        Point::new(100.0, 80.0),
        CHART_W,
        CHART_H,
        SURFACE_H,
    );
    assert_eq!(state.draft_anchors.len(), 1);

    let action = action_or_panic(
        chart.handle_drawing_key_pressed(
            &mut state,
            keyboard::Key::Named(keyboard::key::Named::Escape),
            keyboard::Modifiers::default(),
        ),
        "escape cancels draft",
    );
    let (message, _, _) = action.into_inner();
    assert!(matches!(message, Some(Message::ClearDrawingTool(_, _))));
    assert!(state.draft_anchors.is_empty(), "draft cleared on escape");
    assert_eq!(state.draft_tool, None);
}

#[test]
fn select_press_on_level_selects_and_starts_move_drag() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::Select);
    chart.annotations.push(level(5, 105.0));
    let mut state = ChartState::default();
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(&state, CHART_W, CHART_H)
        .expect("visible price params");
    let y = chart.price_to_y_with(105.0, price_hi, price_range, price_h);

    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(120.0, y),
            CHART_W,
            CHART_H,
            SURFACE_H,
        ),
        "select press should pick the level",
    );
    let (message, _, status) = action.into_inner();
    assert!(matches!(
        message,
        Some(Message::SelectAnnotation(_, Some(5)))
    ));
    assert_eq!(status, Status::Captured);
    assert_eq!(state.selected_annotation, Some(5));
    assert!(matches!(
        state.drag,
        Some(DragKind::MoveAnnotation { id: 5 })
    ));
    assert!(state.drag_annotation_base.is_some());
}

#[test]
fn select_press_on_empty_space_deselects() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::Select);
    chart.annotations.push(level(5, 105.0));
    let mut state = ChartState {
        selected_annotation: Some(5),
        ..ChartState::default()
    };

    // Click far from the level (top of the plot).
    let action = action_or_panic(
        chart.handle_left_press(
            &mut state,
            Point::new(120.0, 5.0),
            CHART_W,
            CHART_H,
            SURFACE_H,
        ),
        "select press on empty space",
    );
    let (message, _, _) = action.into_inner();
    assert!(matches!(message, Some(Message::SelectAnnotation(_, None))));
    assert_eq!(state.selected_annotation, None);
    assert!(state.drag.is_none());
}

#[test]
fn releasing_annotation_drag_publishes_translated_update() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::Select);
    let original = level(5, 105.0);
    chart.annotations.push(original.clone());
    let mut moved = original.clone();
    moved.kind.translate(0, 3.0);

    let mut state = ChartState {
        drag: Some(DragKind::MoveAnnotation { id: 5 }),
        drag_start: Some(Point::new(120.0, 80.0)),
        drag_annotation_base: Some(original),
        drag_annotation: Some(moved),
        selected_annotation: Some(5),
        ..ChartState::default()
    };

    let action = action_or_panic(
        chart.handle_left_release(&mut state, chart_bounds(SURFACE_W, SURFACE_H)),
        "release should commit the moved annotation",
    );
    let (message, _, _) = action.into_inner();
    match message_or_panic(message, "update annotation") {
        Message::UpdateAnnotation(id, ann) => {
            assert_eq!(id, chart.id);
            assert_eq!(ann.id, 5);
            assert!(matches!(
                ann.kind,
                AnnotationKind::HorizontalLevel { price } if (price - 108.0).abs() < 1e-9
            ));
        }
        other => panic!("expected UpdateAnnotation, got {other:?}"),
    }
    assert!(state.drag.is_none());
    assert!(state.drag_annotation.is_none());
    assert!(state.drag_annotation_base.is_none());
}

fn locked_level(id: u64, price: f64) -> Annotation {
    Annotation {
        id,
        kind: AnnotationKind::HorizontalLevel { price },
        style: AnnotationStyle {
            locked: true,
            ..AnnotationStyle::default()
        },
    }
}

fn press_on_level(chart: &CandlestickChart, state: &mut ChartState, price: f64) -> Option<Message> {
    let (price_hi, price_range, price_h) = chart
        .visible_price_params(state, CHART_W, CHART_H)
        .expect("visible price params");
    let y = chart.price_to_y_with(price, price_hi, price_range, price_h);
    let action = action_or_panic(
        chart.handle_left_press(state, Point::new(120.0, y), CHART_W, CHART_H, SURFACE_H),
        "press on level",
    );
    action.into_inner().0
}

#[test]
fn locked_annotation_is_selectable_but_not_draggable() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::Select);
    chart.annotations.push(locked_level(5, 105.0));
    let mut state = ChartState::default();

    let message = press_on_level(&chart, &mut state, 105.0);
    assert!(matches!(
        message,
        Some(Message::SelectAnnotation(_, Some(5)))
    ));
    assert_eq!(state.selected_annotation, Some(5));
    // Locked: selected for unlock, but no move drag started.
    assert!(state.drag.is_none());
    assert!(state.drag_annotation_base.is_none());
}

#[test]
fn eraser_skips_locked_annotation() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::Eraser);
    chart.annotations.push(locked_level(5, 105.0));
    let mut state = ChartState::default();

    let message = press_on_level(&chart, &mut state, 105.0);
    assert!(
        !matches!(message, Some(Message::RemoveAnnotation(_, _))),
        "eraser must not delete a locked drawing, got {message:?}"
    );

    // An unlocked drawing in the same spot is still erasable.
    chart.annotations.clear();
    chart.annotations.push(level(7, 105.0));
    let message = press_on_level(&chart, &mut state, 105.0);
    assert!(matches!(message, Some(Message::RemoveAnnotation(_, 7))));
}

fn delete_key(chart: &CandlestickChart, state: &mut ChartState) -> Option<Message> {
    chart
        .handle_drawing_key_pressed(
            state,
            keyboard::Key::Named(keyboard::key::Named::Delete),
            keyboard::Modifiers::default(),
        )
        .and_then(|action| action.into_inner().0)
}

#[test]
fn delete_key_removes_selection_in_select_mode() {
    let mut chart = chart_with_input_candles();
    chart.active_tool = Some(DrawingTool::Select);
    chart.annotations.push(level(5, 105.0));
    let mut state = ChartState {
        selected_annotation: Some(5),
        ..ChartState::default()
    };

    let message = delete_key(&chart, &mut state);
    assert!(matches!(message, Some(Message::RemoveAnnotation(_, 5))));
    assert_eq!(state.selected_annotation, None);
}

#[test]
fn delete_key_ignored_outside_select_mode() {
    let mut chart = chart_with_input_candles();
    // A stale selection lingers after switching away from the Select tool.
    chart.active_tool = Some(DrawingTool::TrendLine);
    chart.annotations.push(level(5, 105.0));
    let mut state = ChartState {
        selected_annotation: Some(5),
        ..ChartState::default()
    };

    let message = delete_key(&chart, &mut state);
    assert!(
        message.is_none(),
        "Delete must not fire on a stale selection outside Select mode, got {message:?}"
    );
    assert_eq!(state.selected_annotation, Some(5), "selection preserved");
}
