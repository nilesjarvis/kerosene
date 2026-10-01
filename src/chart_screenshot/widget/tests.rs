use super::*;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartSurfaceId;

#[test]
fn capture_targets_the_clicked_surface_and_keeps_fractional_bounds() {
    let docked = TradingTerminal::chart_screenshot_canvas_id(ChartSurfaceId::Docked(7));
    let detached = TradingTerminal::chart_screenshot_canvas_id(ChartSurfaceId::Detached(
        iced::window::Id::unique(),
    ));
    let mut capture = CaptureChartCanvas::new(detached.clone());
    let mut state = ChartState::default();
    capture.custom(
        Some(&docked),
        Rectangle::with_size(Size::new(100.0, 200.0)),
        &mut state,
    );
    assert!(matches!(capture.finish(), Outcome::Some(None)));
    let bounds = Rectangle {
        x: 51.25,
        y: 83.0,
        width: 800.25,
        height: 500.75,
    };
    capture.custom(Some(&detached), bounds, &mut state);
    let Outcome::Some(Some(snapshot)) = capture.finish() else {
        panic!("captured surface")
    };
    assert_eq!(snapshot.size, bounds.size());
    assert!(!std::ptr::eq(snapshot.state.as_ref(), &state));
    capture.custom(
        Some(&docked),
        Rectangle::with_size(Size::new(1.0, 1.0)),
        &mut state,
    );
    let Outcome::Some(Some(snapshot)) = capture.finish() else {
        panic!("retained surface")
    };
    assert_eq!(snapshot.size, bounds.size());
}

#[test]
fn wrapper_preserves_the_canvas_tree_state() {
    let chart = CandlestickChart::new(7);
    let canvas: Element<'_, Message> = Canvas::new(&chart).width(Fill).height(Fill).into();
    let mut tree = tree::Tree::new(&canvas);
    let state_address = tree.state.downcast_ref::<ChartState>() as *const ChartState;
    let wrapper: Element<'_, Message> = ScreenshotCanvas::new(&chart, Id::from("chart")).into();
    tree.diff(&wrapper);
    assert_eq!(
        state_address,
        tree.state.downcast_ref::<ChartState>() as *const ChartState
    );
    assert!(tree.children.is_empty());
}
