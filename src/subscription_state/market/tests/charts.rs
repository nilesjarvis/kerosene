use super::*;
use iced::advanced::subscription::{Hasher, into_recipes};
use std::hash::Hasher as _;

mod funding_live;

fn subscription_hashes(subscription: Subscription<Message>) -> Vec<u64> {
    into_recipes(subscription)
        .into_iter()
        .map(|recipe| {
            let mut hasher = Hasher::default();
            recipe.hash(&mut hasher);
            hasher.finish()
        })
        .collect()
}

#[test]
fn chart_funding_uses_native_asset_context_with_either_read_provider() {
    for provider in [ReadDataProvider::Hyperliquid, ReadDataProvider::Hydromancer] {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal.read_data_provider = provider;
        terminal.hydromancer_api_key = "test-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        for (id, symbol) in [(1, "BTC"), (2, "BTC"), (3, "xyz:NVDA")] {
            terminal.charts.insert(
                id,
                ChartInstance::new(id, symbol.to_string(), Timeframe::H1),
            );
        }
        let source_context = terminal.market_data_source_context();
        let mut subscriptions = Vec::new();
        terminal.push_chart_market_subscriptions(&mut subscriptions);
        let actual = subscription_hashes(Subscription::batch(subscriptions));
        assert_eq!(actual.len(), 4, "one candle and context stream per symbol");

        for (id, symbol) in [(1, "BTC"), (3, "xyz:NVDA")] {
            // Hydromancer's activeAssetCtx omits funding. The chart header
            // must use Hyperliquid's complete context even with that provider.
            let expected =
                Subscription::run_with((id, symbol.to_string()), ws_asset_ctx_stream_keyed)
                    .with(source_context)
                    .map(chart_asset_ctx_stream_event_message);
            for hash in subscription_hashes(expected) {
                assert!(
                    actual.contains(&hash),
                    "native funding stream for {provider:?}"
                );
            }
        }
    }
}

#[test]
fn native_chart_funding_updates_all_matching_charts_with_hydromancer_selected() {
    for funding in ["0.0000125", "0", "-0.0001"] {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "test-key".to_string().into();
        terminal.hydromancer_key_generation = 2;
        for (id, symbol) in [(1, "BTC"), (2, "BTC"), (3, "ETH")] {
            terminal.charts.insert(
                id,
                ChartInstance::new(id, symbol.to_string(), Timeframe::H1),
            );
        }
        let source_context = terminal.market_data_source_context();
        let ctx = serde_json::from_value(serde_json::json!({ "funding": funding }))
            .expect("native asset context");
        let message = chart_asset_ctx_stream_event_message((
            source_context,
            crate::ws::KeyedAssetContextStreamEvent::Item(1, "BTC".into(), None, Box::new(ctx)),
        ));
        let _task = terminal.update_chart(message.clone());
        for id in [1, 2] {
            let ctx = terminal.charts[&id]
                .asset_ctx
                .as_ref()
                .expect("chart context");
            assert_eq!(ctx.funding.as_deref(), Some(funding));
        }
        assert!(terminal.charts[&3].asset_ctx.is_none());

        // A provider switch still invalidates messages from the old stream.
        terminal.read_data_provider_generation += 1;
        for chart in terminal.charts.values_mut() {
            chart.set_asset_context(None);
        }
        let _task = terminal.update_chart(message);
        assert!(
            terminal
                .charts
                .values()
                .all(|chart| chart.asset_ctx.is_none())
        );
    }
}

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

    // Candle streaming starts before REST history so a slow cold load cannot
    // leave the chart displaying an unqualified stale snapshot.
    assert_eq!(subscriptions.len(), 2);
}

#[test]
fn loading_chart_subscribes_to_live_candles_before_history_arrives() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));

    let mut subscriptions = Vec::new();
    terminal.push_chart_market_subscriptions(&mut subscriptions);

    // One candle stream and one asset-context stream.
    assert_eq!(subscriptions.len(), 2);
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

#[test]
fn chart_asset_context_lagged_event_preserves_fallback_generation_scope() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-secret".to_string().into();
    terminal.hydromancer_key_generation = 2;
    let source_context = terminal.market_data_source_context();
    assert_eq!(source_context.hydromancer_key_generation, Some(2));

    let message = chart_asset_ctx_stream_event_message((
        source_context,
        crate::ws::KeyedAssetContextStreamEvent::Lagged {
            id: 7,
            symbol: "BTC".to_string(),
            hydromancer_key_generation: None,
            skipped: 9,
        },
    ));

    match message {
        Message::ChartWsAssetCtxLagged(_, _, mapped_context, _) => {
            assert_eq!(
                mapped_context,
                crate::read_data_provider::MarketDataSourceContext {
                    hydromancer_key_generation: None,
                    ..source_context
                }
            );
        }
        other => panic!("expected chart asset-context lagged message, got {other:?}"),
    }
}
