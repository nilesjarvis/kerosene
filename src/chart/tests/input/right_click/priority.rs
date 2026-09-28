use super::*;
use crate::annotations::DrawingTool;
use crate::chart::fisheye::ChartFisheye;
use crate::chart::interaction::{InteractionLayout, ProjectedCursor};
use crate::chart_state::ChartSurfaceId;
use iced::event::Status;
use iced::window;

#[test]
fn right_click_preserves_quick_order_priority_and_missing_price_fallthrough() {
    for surface in [
        ChartSurfaceId::Docked(1),
        ChartSurfaceId::Detached(window::Id::unique()),
    ] {
        for loaded in [false, true] {
            for open in [false, true] {
                for obstruction in 0..5 {
                    let mut chart = quick_order_chart();
                    chart.surface_id = surface;
                    chart.quick_order_open = open;
                    if !loaded {
                        chart.candles.clear();
                    }
                    if obstruction == 2 {
                        chart.active_tool = Some(DrawingTool::TrendLine);
                    }
                    if obstruction == 3 {
                        chart.active_orders.push(btc_buy_order(42));
                    }
                    if obstruction == 4 {
                        chart.active_orders.push(pending_btc_buy_order(42));
                    }
                    let mut state = ChartState {
                        range_anchor_price: (obstruction == 1).then_some(101.0),
                        ..Default::default()
                    };
                    let y = chart
                        .visible_price_params(&state, CHART_W, CHART_H)
                        .map_or(100.0, |(hi, range, height)| {
                            chart.price_to_y_with(105.0, hi, range, height)
                        });
                    let source = Point::new(260.0, y);
                    let action = chart.handle_right_press_at(
                        &mut state,
                        chart_bounds(SURFACE_W, SURFACE_H),
                        ProjectedCursor {
                            source,
                            visual: Point::new(280.0, 80.0),
                        },
                        ChartFisheye::disabled(),
                        InteractionLayout::without_funding(CHART_W, CHART_H),
                    );
                    if open && loaded {
                        let (message, _, status) =
                            action.expect("replacement quick order").into_inner();
                        assert_open_quick_order_message(message, &chart, source);
                        assert_eq!(status, Status::Captured);
                        assert_eq!(
                            state.range_anchor_price,
                            (obstruction == 1).then_some(101.0)
                        );
                    } else if obstruction == 1 {
                        let (message, _, status) = action.expect("clear range").into_inner();
                        assert!(message.is_none());
                        assert_eq!(status, Status::Ignored);
                        assert!(state.range_anchor_price.is_none());
                    } else if obstruction == 2 {
                        let (message, _, status) = action.expect("clear tool").into_inner();
                        assert!(
                            matches!(message, Some(Message::ClearDrawingTool(1, actual_surface)) if actual_surface == surface)
                        );
                        assert_eq!(status, Status::Captured);
                    } else if obstruction == 3 && loaded {
                        let (message, _, status) = action.expect("cancel live order").into_inner();
                        assert!(
                            matches!(message, Some(Message::CancelOrder { coin, oid: 42 }) if coin == "BTC")
                        );
                        assert_eq!(status, Status::Captured);
                    } else if loaded {
                        let (message, _, status) = action.expect("open quick order").into_inner();
                        assert_open_quick_order_message(message, &chart, source);
                        assert_eq!(status, Status::Captured);
                    } else {
                        assert!(action.is_none());
                    }
                    assert!(state.drag.is_none());
                }
            }
        }
    }
}

#[test]
fn quick_order_initial_and_replacement_clicks_share_source_price_clamping() {
    for open in [false, true] {
        for source_y in [-25.0, CHART_H - 1.0] {
            let mut chart = quick_order_chart();
            chart.quick_order_open = open;
            chart.surface_id = ChartSurfaceId::Detached(window::Id::unique());
            let mut state = ChartState::default();
            let (hi, range, height) = chart
                .visible_price_params(&state, CHART_W, CHART_H)
                .expect("price range");
            let source = Point::new(90.0, source_y);
            let action = chart
                .handle_right_press_at(
                    &mut state,
                    chart_bounds(SURFACE_W, SURFACE_H),
                    ProjectedCursor {
                        source,
                        visual: Point::new(120.0, 80.0),
                    },
                    ChartFisheye::disabled(),
                    InteractionLayout::without_funding(CHART_W, CHART_H),
                )
                .expect("quick-order action");
            let (message, _, status) = action.into_inner();
            let Some(Message::OpenQuickOrder(id, surface, price, x, y, width, chart_height)) =
                message
            else {
                panic!("quick-order payload")
            };
            assert_eq!(id, chart.id);
            assert_eq!(surface, chart.surface_id);
            assert_eq!(
                price,
                chart.y_to_price_with(source_y.clamp(0.0, height), hi, range, height)
            );
            assert_eq!((x, y), (source.x, source.y));
            assert_eq!((width, chart_height), (CHART_W, CHART_H));
            assert_eq!(status, Status::Captured);
        }
    }
}
