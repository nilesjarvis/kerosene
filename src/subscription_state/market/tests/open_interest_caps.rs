use super::*;

#[test]
fn cap_status_has_one_stable_native_subscription_for_all_visible_dexes() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.panes = pane_grid::State::new(PaneKind::Chart(1)).0;
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    let expected = subscription_hashes(
        Subscription::run(crate::ws::ws_open_interest_caps)
            .map(Message::ChartOpenInterestCapStream),
    );
    let mut subscriptions = Vec::new();
    terminal.push_chart_open_interest_cap_subscription(&mut subscriptions);
    assert_eq!(
        subscription_hashes(Subscription::batch(subscriptions)),
        expected
    );

    terminal.insert_test_canvas_pane(7, PaneKind::Chart(2));
    terminal.charts.insert(
        2,
        ChartInstance::new(2, "xyz:AAPL".to_string(), Timeframe::H1),
    );
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "fixture-key".into();
    let mut subscriptions = Vec::new();
    terminal.push_chart_open_interest_cap_subscription(&mut subscriptions);
    assert_eq!(
        subscription_hashes(Subscription::batch(subscriptions)),
        expected
    );
}

#[test]
fn cap_status_subscription_skips_spot_outcomes_hidden_and_collapsed_charts() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.panes = pane_grid::State::new(PaneKind::Chart(1)).0;
    for (symbol, collapsed, hidden) in [
        ("@107", false, false),
        ("#1", false, false),
        ("BTC", true, false),
        ("BTC", false, true),
    ] {
        let mut chart = ChartInstance::new(1, symbol.to_string(), Timeframe::H1);
        chart.header_collapsed = collapsed;
        terminal.charts.insert(1, chart);
        terminal.muted_tickers.clear();
        if hidden {
            terminal.muted_tickers.insert(symbol.to_string());
        }
        let mut subscriptions = Vec::new();
        terminal.push_chart_open_interest_cap_subscription(&mut subscriptions);
        assert!(subscriptions.is_empty());
    }
}
