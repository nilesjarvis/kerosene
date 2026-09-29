use super::{SURFACE_H, SURFACE_W, chart_bounds, chart_with_input_candles};
use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle};
use crate::chart::state::DragKind;
use crate::chart::{CandlestickChart, ChartState};
use crate::chart_state::ChartSurfaceId;
use crate::message::Message;
use iced::advanced::renderer::Headless;
use iced::event::Status;
use iced::{Font, Pixels, Point, Renderer, Size, mouse, window};

fn annotation(price: f64) -> Annotation {
    Annotation {
        id: 7,
        kind: AnnotationKind::HorizontalLevel { price },
        style: AnnotationStyle {
            label: Some("synthetic drag".into()),
            ..AnnotationStyle::default()
        },
    }
}

#[tokio::test]
async fn drag_release_preserves_payload_cleanup_capture_and_cache_policies() {
    let renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let bounds = chart_bounds(SURFACE_W, SURFACE_H);
    let kinds = [
        None,
        Some(DragKind::MoveOrder { oid: 42 }),
        Some(DragKind::MoveAnnotation { id: 7 }),
        Some(DragKind::MoveAnnotationAnchor {
            id: 7,
            anchor_index: 0,
        }),
        Some(DragKind::PanX),
        Some(DragKind::PanY),
        Some(DragKind::PanFundingY),
        Some(DragKind::ResizeFundingPanel),
        Some(DragKind::ResizeSessionPanel),
    ];
    for surface in [
        ChartSurfaceId::Docked(1),
        ChartSurfaceId::Detached(window::Id::unique()),
    ] {
        for case in 0..3 {
            for kind in kinds {
                let mut chart = if case == 1 {
                    CandlestickChart::new(1)
                } else {
                    chart_with_input_candles()
                };
                chart.surface_id = surface;
                let price = [Some(123.0), None, Some(f64::NAN)][case];
                let funding_height = [Some(66.6), None, Some(f32::NAN)][case];
                let session_height = [Some(77.4), None, Some(f32::NAN)][case];
                let mut state = ChartState {
                    drag: kind,
                    drag_start: Some(Point::new(120.0, 80.0)),
                    drag_order_coin: (case != 1).then(|| "SYNTH".into()),
                    drag_order_new_price: price,
                    drag_annotation_base: Some(annotation(100.0)),
                    drag_annotation: price.map(annotation),
                    drag_funding_panel_height: funding_height,
                    drag_session_panel_height: session_height,
                    selected_annotation: Some(7),
                    range_anchor_price: Some(99.0),
                    ..Default::default()
                };
                let size = Size::new(10.0, 10.0);
                let _ = chart.candle_cache.draw(&renderer, size, |_| {});
                let action = chart.handle_left_release(&mut state, bounds);
                let mut rebuilt = false;
                let _ = chart.candle_cache.draw(&renderer, size, |_| rebuilt = true);
                assert_eq!(
                    rebuilt,
                    matches!(
                        kind,
                        Some(DragKind::PanX | DragKind::PanY | DragKind::PanFundingY)
                    )
                );

                let order_drag = matches!(kind, Some(DragKind::MoveOrder { .. }));
                let annotation_drag = matches!(
                    kind,
                    Some(DragKind::MoveAnnotation { .. } | DragKind::MoveAnnotationAnchor { .. })
                );
                let panel_cleanup = kind.is_some() && !order_drag && !annotation_drag;
                assert!(state.drag.is_none());
                assert_eq!(state.drag_start.is_some(), kind.is_none());
                assert_eq!(
                    state.drag_order_coin.as_deref(),
                    (!order_drag && case != 1).then_some("SYNTH")
                );
                assert_eq!(
                    state.drag_order_new_price.map(f64::to_bits),
                    (!order_drag).then_some(price).flatten().map(f64::to_bits)
                );
                assert_eq!(state.drag_annotation_base.is_some(), !annotation_drag);
                assert_eq!(
                    state.drag_annotation.is_some(),
                    !annotation_drag && case != 1
                );
                assert_eq!(
                    state.drag_funding_panel_height.map(f32::to_bits),
                    (!panel_cleanup)
                        .then_some(funding_height)
                        .flatten()
                        .map(f32::to_bits)
                );
                assert_eq!(
                    state.drag_session_panel_height.map(f32::to_bits),
                    (!panel_cleanup)
                        .then_some(session_height)
                        .flatten()
                        .map(f32::to_bits)
                );
                assert_eq!(state.selected_annotation, Some(7));
                assert_eq!(state.range_anchor_price, Some(99.0));

                let Some(kind) = kind else {
                    assert!(action.is_none());
                    continue;
                };
                let (message, _, status) = action.expect("active drag release").into_inner();
                assert_eq!(
                    status,
                    if message.is_some() {
                        Status::Captured
                    } else {
                        Status::Ignored
                    }
                );
                match kind {
                    DragKind::MoveOrder { oid } if case != 1 => {
                        let Some(Message::MoveOrder {
                            coin,
                            oid: actual_oid,
                            new_price,
                        }) = message
                        else {
                            panic!("move order payload")
                        };
                        assert_eq!(coin, "SYNTH");
                        assert_eq!(actual_oid, oid);
                        assert_eq!(Some(new_price.to_bits()), price.map(f64::to_bits));
                    }
                    DragKind::MoveAnnotation { .. } | DragKind::MoveAnnotationAnchor { .. }
                        if case == 0 =>
                    {
                        let Some(Message::UpdateAnnotation(id, live)) = message else {
                            panic!("annotation payload")
                        };
                        assert_eq!(id, chart.id);
                        assert_eq!(live.id, 7);
                        assert_eq!(live.kind, AnnotationKind::HorizontalLevel { price: 123.0 });
                        assert_eq!(live.style.label.as_deref(), Some("synthetic drag"));
                    }
                    DragKind::ResizeFundingPanel => {
                        assert!(
                            matches!(message, Some(Message::ChartFundingPanelHeightChanged(1, height, true)) if height == [67, 56, 0][case])
                        );
                    }
                    DragKind::ResizeSessionPanel => {
                        assert!(
                            matches!(message, Some(Message::ChartSessionPanelHeightChanged(1, height, true)) if height == [77, 56, 0][case])
                        );
                    }
                    DragKind::PanX | DragKind::PanY if case != 1 => {
                        let Some(Message::ChartViewportChanged(id, actual_surface, viewport)) =
                            message
                        else {
                            panic!("viewport payload")
                        };
                        assert_eq!(id, chart.id);
                        assert_eq!(actual_surface, surface);
                        assert_eq!(
                            (viewport.start_time_ms, viewport.end_time_ms),
                            (1_000, 2_000)
                        );
                    }
                    _ => assert!(message.is_none()),
                }
            }
        }
    }
}

#[test]
fn annotation_drag_uses_the_original_snapshot_and_retains_preview_without_inputs() {
    for anchor_drag in [false, true] {
        let mut chart = chart_with_input_candles();
        chart.fisheye_enabled = false;
        let bounds = chart_bounds(SURFACE_W, SURFACE_H);
        let chart_w = bounds.width - chart.price_axis_width();
        let (chart_h, _, _) = chart.chart_area_heights(bounds.height);
        let start = Point::new(120.0, 80.0);
        let base = annotation(105.0);
        let mut state = ChartState {
            drag: Some(if anchor_drag {
                DragKind::MoveAnnotationAnchor {
                    id: 7,
                    anchor_index: 0,
                }
            } else {
                DragKind::MoveAnnotation { id: 7 }
            }),
            drag_start: Some(start),
            drag_annotation_base: Some(base.clone()),
            drag_annotation: Some(base.clone()),
            ..Default::default()
        };
        let (price_hi, price_range, price_h) = chart
            .visible_price_params(&state, chart_w, chart_h)
            .expect("price range");
        let start_price = chart.y_to_price_with(start.y, price_hi, price_range, price_h);
        for pos in [Point::new(120.0, 110.0), Point::new(120.0, 50.0)] {
            let price = chart.y_to_price_with(pos.y, price_hi, price_range, price_h);
            chart.update_interaction(
                &mut state,
                &iced::Event::Mouse(mouse::Event::CursorMoved { position: pos }),
                bounds,
                mouse::Cursor::Available(pos),
            );
            let live = state.drag_annotation.as_ref().expect("live annotation");
            let expected = if anchor_drag {
                price
            } else {
                105.0 + (price - start_price)
            };
            assert_eq!(
                live.kind,
                AnnotationKind::HorizontalLevel { price: expected }
            );
            assert_eq!(live.style, base.style);
            assert_eq!(
                state.drag_annotation_base.as_ref().expect("base").kind,
                base.kind
            );
        }

        let last_kind = state
            .drag_annotation
            .as_ref()
            .expect("preview")
            .kind
            .clone();
        chart.candles.clear();
        let pos = Point::new(120.0, 150.0);
        chart.update_interaction(
            &mut state,
            &iced::Event::Mouse(mouse::Event::CursorMoved { position: pos }),
            bounds,
            mouse::Cursor::Available(pos),
        );
        assert_eq!(
            state
                .drag_annotation
                .as_ref()
                .expect("retained preview")
                .kind,
            last_kind
        );
        assert_eq!(
            state
                .drag_annotation_base
                .as_ref()
                .expect("retained base")
                .kind,
            base.kind
        );
        assert!(state.drag.is_some());
    }
}
