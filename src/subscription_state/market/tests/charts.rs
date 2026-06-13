use super::*;

#[test]
fn duplicate_chart_market_streams_are_deduplicated_by_market_key() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut btc_h1_primary = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    btc_h1_primary.chart.status = ChartStatus::Loaded;
    let mut btc_h1_detached = ChartInstance::new(2, "BTC".to_string(), Timeframe::H1);
    btc_h1_detached.chart.status = ChartStatus::Loaded;
    let mut btc_m5 = ChartInstance::new(3, "BTC".to_string(), Timeframe::M5);
    btc_m5.chart.status = ChartStatus::Loaded;
    let mut eth_h1 = ChartInstance::new(4, "ETH".to_string(), Timeframe::H1);
    eth_h1.chart.status = ChartStatus::Loaded;

    terminal.charts.insert(1, btc_h1_primary);
    terminal.charts.insert(2, btc_h1_detached);
    terminal.charts.insert(3, btc_m5);
    terminal.charts.insert(4, eth_h1);

    let mut subscriptions = Vec::new();
    terminal.push_chart_market_subscriptions(&mut subscriptions);

    assert_eq!(subscriptions.len(), 5);
}

#[test]
fn outcome_charts_subscribe_to_asset_context_for_header_metrics() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.exchange_symbols = vec![exchange_symbol("#650", MarketType::Outcome)];

    terminal
        .charts
        .insert(1, ChartInstance::new(1, "#650".to_string(), Timeframe::H1));

    let mut subscriptions = Vec::new();
    terminal.push_chart_market_subscriptions(&mut subscriptions);

    assert_eq!(subscriptions.len(), 1);
}

#[test]
fn chart_asset_context_lagged_event_maps_to_chart_message() {
    let terminal = TradingTerminal::boot().0;
    let source_context = terminal.market_data_source_context();

    let message = chart_asset_ctx_stream_event_message((
        source_context,
        crate::ws::KeyedAssetContextStreamEvent::Lagged {
            id: 7,
            symbol: "BTC".to_string(),
            hydromancer_key_generation: source_context.hydromancer_key_generation,
            skipped: 9,
        },
    ));

    match message {
        Message::ChartWsAssetCtxLagged(id, symbol, mapped_context, skipped) => {
            assert_eq!(id, 7);
            assert_eq!(symbol, "BTC");
            assert_eq!(mapped_context, source_context);
            assert_eq!(skipped, 9);
        }
        other => panic!("expected chart asset-context lagged message, got {other:?}"),
    }
}
