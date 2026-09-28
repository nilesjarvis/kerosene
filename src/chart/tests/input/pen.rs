use super::{btc_buy_order, candle_at};
use crate::annotations::{Annotation, AnnotationKind, DrawingTool, MAX_PEN_POINTS};
use crate::chart::state::DragKind;
use crate::chart::tests::chart_bounds;
use crate::chart::{CandlestickChart, ChartState};
use crate::message::Message;
use iced::{Event, Point, event::Status, keyboard, mouse, window};

fn pen_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(7);
    chart.set_candles(vec![
        candle_at(1_700_000_000_000, 100.0),
        candle_at(1_700_000_060_000, 110.0),
    ]);
    chart.active_tool = Some(DrawingTool::Pen);
    chart
}

fn send(
    chart: &CandlestickChart,
    state: &mut ChartState,
    event: Event,
    pos: Option<Point>,
) -> Option<Message> {
    chart
        .update_interaction(
            state,
            &event,
            chart_bounds(600.0, 400.0),
            pos.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
        )
        .and_then(|action| action.into_inner().0)
}

fn press(chart: &CandlestickChart, state: &mut ChartState, pos: Point) {
    let action = chart
        .update_interaction(
            state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            chart_bounds(600.0, 400.0),
            mouse::Cursor::Available(pos),
        )
        .expect("pen press captured");
    let (message, _, status) = action.into_inner();
    assert!(message.is_none(), "press must not commit or place an order");
    assert_eq!(status, Status::Captured);
}

fn move_to(chart: &CandlestickChart, state: &mut ChartState, pos: Point) {
    let _ = send(
        chart,
        state,
        Event::Mouse(mouse::Event::CursorMoved { position: pos }),
        Some(pos),
    );
}

fn release(
    chart: &CandlestickChart,
    state: &mut ChartState,
    pos: Option<Point>,
) -> Option<Annotation> {
    match send(
        chart,
        state,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        pos,
    ) {
        Some(Message::AddAnnotation(id, annotation)) => {
            assert_eq!(id, chart.id);
            Some(annotation)
        }
        None => None,
        other => panic!("unexpected release message: {other:?}"),
    }
}

#[test]
fn pen_drag_commits_on_release_and_keeps_time_price_geometry() {
    let mut chart = pen_chart();
    chart.active_orders.push(btc_buy_order(42));
    let mut state = ChartState::default();
    let chart_w = 600.0 - chart.price_axis_width();
    let (chart_h, _, _) = chart.chart_area_heights(400.0);
    let (hi, range, price_h) = chart
        .visible_price_params(&state, chart_w, chart_h)
        .expect("prices");
    let start = Point::new(100.0, chart.price_to_y_with(105.0, hi, range, price_h));
    let positions = [start, Point::new(180.0, 80.0), Point::new(140.0, 120.0)];
    press(&chart, &mut state, start);
    assert_eq!(state.drag, Some(DragKind::DrawPen));
    move_to(&chart, &mut state, positions[1]);
    // The release location is sampled even without a preceding move event.
    let annotation = release(&chart, &mut state, Some(positions[2])).expect("stroke");
    assert!(annotation.is_valid());
    let AnnotationKind::Pen { points } = &annotation.kind else {
        panic!("pen kind");
    };
    assert_eq!(points.len(), positions.len());
    for (&(time, price), pos) in points.iter().zip(positions) {
        assert!((chart.timestamp_to_x(time, &state, chart_w).expect("x") - pos.x).abs() < 0.01);
        assert!((chart.price_to_y_with(price, hi, range, price_h) - pos.y).abs() < 0.01);
    }
    assert!(
        points[2].0 < points[1].0,
        "backtracking must retain traversal order"
    );
    assert_eq!(state.scroll_offset, 0.0);
    assert!(state.y_auto);
    assert!(state.drag.is_none());
    assert!(state.draft_anchors.is_empty());

    // A second gesture creates an independent stroke with the pen still active.
    press(&chart, &mut state, Point::new(210.0, 80.0));
    move_to(&chart, &mut state, Point::new(230.0, 100.0));
    let second = release(&chart, &mut state, None).expect("release outside canvas saves stroke");
    assert_ne!(second.kind, annotation.kind);
    move_to(&chart, &mut state, Point::new(300.0, 150.0));
    assert!(state.draft_anchors.is_empty(), "hover never adds ink");
}

#[test]
fn pen_cancellation_never_commits_a_stale_stroke() {
    for cancel in 0..4 {
        let mut chart = pen_chart();
        let mut state = ChartState::default();
        let pos = Point::new(100.0, 100.0);
        press(&chart, &mut state, pos);
        move_to(&chart, &mut state, Point::new(150.0, 140.0));
        match cancel {
            0 => {
                let _ = chart.handle_drawing_key_pressed(
                    &mut state,
                    keyboard::Key::Named(keyboard::key::Named::Escape),
                    keyboard::Modifiers::default(),
                );
            }
            1 => {
                let _ = send(
                    &chart,
                    &mut state,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                    Some(pos),
                );
            }
            2 => {
                let _ = send(
                    &chart,
                    &mut state,
                    Event::Window(window::Event::Unfocused),
                    None,
                );
            }
            _ => {
                chart.active_tool = Some(DrawingTool::TrendLine);
                move_to(&chart, &mut state, pos);
            }
        }
        assert!(state.drag.is_none(), "cancel case {cancel}");
        assert!(state.draft_anchors.is_empty(), "cancel case {cancel}");
        assert!(release(&chart, &mut state, Some(pos)).is_none());
    }
}

#[test]
fn pen_filters_jitter_bounds_long_strokes_and_ignores_clicks() {
    let chart = pen_chart();
    let mut state = ChartState::default();
    let pos = Point::new(100.0, 100.0);
    press(&chart, &mut state, pos);
    assert!(
        release(&chart, &mut state, Some(pos)).is_none(),
        "click alone leaves no invisible stroke"
    );
    press(&chart, &mut state, pos);
    move_to(&chart, &mut state, Point::new(100.5, 100.5));
    assert_eq!(state.draft_anchors.len(), 1);
    let first = state.draft_anchors[0];
    for index in 0..MAX_PEN_POINTS * 2 {
        move_to(
            &chart,
            &mut state,
            Point::new(150.0 + (index % 2) as f32 * 20.0, 120.0),
        );
        assert!(state.draft_anchors.len() <= MAX_PEN_POINTS);
    }
    assert_eq!(state.draft_anchors[0], first);
    assert!(
        release(&chart, &mut state, None)
            .expect("bounded stroke")
            .is_valid()
    );
}

#[test]
fn pen_owns_wheel_during_stroke_and_does_not_draw_in_volume_panel() {
    let chart = pen_chart();
    let mut state = ChartState::default();
    let pos = Point::new(100.0, 100.0);
    let width = state.candle_width;
    press(&chart, &mut state, pos);
    let _ = send(
        &chart,
        &mut state,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        }),
        Some(pos),
    );
    assert_eq!(state.candle_width, width);
    assert!(release(&chart, &mut state, Some(pos)).is_none());
    let (chart_h, _, _) = chart.chart_area_heights(400.0);
    press(&chart, &mut state, Point::new(100.0, chart_h - 5.0));
    assert!(state.drag.is_none());
    assert!(state.draft_anchors.is_empty());
}

#[test]
fn pen_stroke_supports_selection_body_drag_and_eraser() {
    let mut chart = pen_chart();
    let mut state = ChartState::default();
    let pos = Point::new(100.0, 100.0);
    press(&chart, &mut state, pos);
    move_to(&chart, &mut state, Point::new(160.0, 100.0));
    let mut stroke = release(&chart, &mut state, None).expect("stroke");
    stroke.id = 5;
    let original = stroke.kind.clone();
    chart.annotations.push(stroke);
    chart.active_tool = Some(DrawingTool::Select);
    assert!(matches!(
        send(
            &chart,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(pos)
        ),
        Some(Message::SelectAnnotation(_, Some(5)))
    ));
    assert_eq!(state.drag, Some(DragKind::MoveAnnotation { id: 5 }));
    move_to(&chart, &mut state, Point::new(130.0, 120.0));
    let message = send(
        &chart,
        &mut state,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        None,
    );
    assert!(matches!(message, Some(Message::UpdateAnnotation(_, ref ann)) if ann.kind != original));
    chart.active_tool = Some(DrawingTool::Eraser);
    let message = send(
        &chart,
        &mut state,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Some(pos),
    );
    assert!(matches!(message, Some(Message::RemoveAnnotation(_, 5))));
    chart.annotations[0].style.locked = true;
    assert!(
        send(
            &chart,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(pos)
        )
        .is_none()
    );
    chart.annotations[0].style.visible = false;
    let chart_w = 600.0 - chart.price_axis_width();
    let (chart_h, _, _) = chart.chart_area_heights(400.0);
    assert!(
        chart
            .hit_test_annotation(&state, pos, chart_w, chart_h)
            .is_none()
    );
}
