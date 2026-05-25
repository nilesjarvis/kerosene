use super::*;

#[test]
fn spaghetti_market_streams_skip_unloaded_empty_or_hidden_series() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.spaghetti_charts.clear();
    terminal.muted_tickers.insert("HIDDEN".to_string());

    let mut instance = SpaghettiChartInstance::new_empty(7);
    instance.canvas = SpaghettiCanvas::new();
    instance.canvas.series = vec![
        spaghetti_series("BTC", true),
        spaghetti_series("ETH", true),
        spaghetti_series("SOL", false),
        spaghetti_series("", true),
        spaghetti_series("HIDDEN", true),
    ];
    terminal.spaghetti_charts.insert(7, instance);

    let mut subscriptions = Vec::new();
    terminal.push_spaghetti_market_subscriptions(&mut subscriptions);

    assert_eq!(subscriptions.len(), 2);
}
