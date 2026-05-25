use super::*;

#[test]
fn market_chart_feed_and_export_routes_stay_on_their_feature_modules() {
    assert_route(
        Message::ClearDrawingTool(7, crate::chart_state::ChartSurfaceId::Docked(7)),
        UpdateRoute::Annotations,
    );
    assert_route(
        Message::HydromancerKeyInputChanged(String::new()),
        UpdateRoute::Feed,
    );
    assert_route(Message::RefreshHeatmap, UpdateRoute::Hyperdash);
    assert_route(Message::OpenDetachedChart(7), UpdateRoute::Chart);
    assert_route(
        Message::OpenChartScreenshot(7, crate::chart_state::ChartSurfaceId::Docked(7)),
        UpdateRoute::ChartScreenshot,
    );
    assert_route(
        Message::ToggleChartScreenshotObscurePositionEntry(true),
        UpdateRoute::ChartScreenshot,
    );
    assert_route(
        Message::ToggleChartScreenshotHidePositionsAndOrders(true),
        UpdateRoute::ChartScreenshot,
    );
    assert_route(
        Message::OutcomeMarketGroupToggled("question:19".to_string()),
        UpdateRoute::Market,
    );
}
