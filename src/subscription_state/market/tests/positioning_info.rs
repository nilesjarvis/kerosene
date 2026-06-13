use super::*;

#[test]
fn positioning_info_market_streams_require_visible_supported_symbols_and_key() {
    let mut terminal = TradingTerminal::boot().0;
    let (mut panes, root_pane) = pane_grid::State::new(PaneKind::PositioningInfo(1));
    split_positioning_pane(&mut panes, root_pane, pane_grid::Axis::Vertical, 2);
    split_positioning_pane(&mut panes, root_pane, pane_grid::Axis::Horizontal, 3);
    split_positioning_pane(&mut panes, root_pane, pane_grid::Axis::Vertical, 4);
    split_positioning_pane(&mut panes, root_pane, pane_grid::Axis::Horizontal, 5);
    terminal.panes = panes;
    terminal.positioning_infos.clear();
    terminal.exchange_symbols = vec![exchange_symbol("BTC", MarketType::Perp)];
    terminal.muted_tickers.insert("HIDDEN".to_string());

    terminal
        .positioning_infos
        .insert(1, PositioningInfoInstance::new(1, "BTC".to_string()));
    terminal
        .positioning_infos
        .insert(2, PositioningInfoInstance::new(2, "ETH".to_string()));
    terminal
        .positioning_infos
        .insert(3, PositioningInfoInstance::new(3, "BTC".to_string()));
    terminal
        .positioning_infos
        .insert(4, PositioningInfoInstance::new(4, "HIDDEN".to_string()));
    terminal
        .positioning_infos
        .insert(5, PositioningInfoInstance::new(5, "@1".to_string()));

    let mut subscriptions = Vec::new();
    terminal.push_positioning_info_market_subscriptions(&mut subscriptions);
    assert!(subscriptions.is_empty());

    terminal.hyperdash_api_key = crate::app_state::sensitive_string("test-key");

    terminal.push_positioning_info_market_subscriptions(&mut subscriptions);
    assert_eq!(subscriptions.len(), 2);
}

#[test]
fn positioning_asset_context_lagged_event_maps_to_market_message() {
    let terminal = TradingTerminal::boot().0;
    let source_context = terminal.market_data_source_context();

    let message = positioning_asset_ctx_stream_event_message((
        source_context,
        crate::ws::SymbolAssetContextStreamEvent::Lagged {
            symbol: "BTC".to_string(),
            hydromancer_key_generation: source_context.hydromancer_key_generation,
            skipped: 9,
        },
    ));

    match message {
        Message::PositioningInfoWsAssetCtxLagged(symbol, mapped_context, skipped) => {
            assert_eq!(symbol, "BTC");
            assert_eq!(mapped_context, source_context);
            assert_eq!(skipped, 9);
        }
        other => panic!("expected positioning asset-context lagged message, got {other:?}"),
    }
}
