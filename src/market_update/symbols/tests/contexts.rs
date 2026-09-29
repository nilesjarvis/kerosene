use super::*;

#[test]
fn partial_context_refresh_filters_old_and_new_out_of_scope_values() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.symbol_search_ctxs = HashMap::from([
        ("BTC".into(), context(1.0)),
        ("ETH".into(), context(2.0)),
        ("OLD".into(), context(3.0)),
    ]);
    terminal.symbol_search_contexts_request_id = 7;
    terminal.symbol_search_contexts_loading = true;
    terminal.symbol_search_contexts_refresh_pending = false;
    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        7,
        vec!["BTC".into(), "ETH".into(), "missing".into()],
        42,
        Ok(crate::api::WatchlistContextsResponse {
            contexts: HashMap::from([("BTC".into(), context(9.0)), ("NEW".into(), context(4.0))]),
            partial_errors: vec!["perps unavailable".into()],
        }),
    ));
    assert_eq!(terminal.symbol_search_ctxs.len(), 2);
    assert_eq!(terminal.symbol_search_ctxs["BTC"].day_vlm, Some(9.0));
    assert_eq!(terminal.symbol_search_ctxs["ETH"].day_vlm, Some(2.0));
    assert_eq!(terminal.symbol_search_contexts_request_id, 8);
    assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(42));
    assert!(!terminal.symbol_search_contexts_loading);
    assert!(terminal.symbol_search_contexts_request_symbols.is_empty());
    assert!(
        terminal
            .symbol_search_status
            .as_ref()
            .is_some_and(|(_, error)| *error)
    );
}

#[test]
fn symbol_search_context_filter_change_queues_current_scope_after_in_flight_result() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC"), perp_symbol("xyz:ETH")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
    terminal.symbol_search_market_filter = SymbolSearchMarketFilter::NativePerps;

    let _task = terminal.request_symbol_search_context_refresh(true);
    let stale_request_id = terminal.symbol_search_contexts_request_id;
    assert!(terminal.symbol_search_contexts_loading);
    assert_eq!(
        terminal.symbol_search_contexts_request_symbols,
        vec!["BTC".to_string()]
    );

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchMarketFilterChanged(
        SymbolSearchMarketFilter::Hip3,
    ));
    assert!(terminal.symbol_search_contexts_refresh_pending);

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        stale_request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(
        terminal.symbol_search_contexts_loading,
        "queued refresh should start for the current HIP-3 scope"
    );
    assert!(!terminal.symbol_search_contexts_refresh_pending);
    assert_eq!(
        terminal.symbol_search_contexts_request_symbols,
        vec!["xyz:ETH".to_string()]
    );
    let current_request_id = terminal.symbol_search_contexts_request_id;

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        current_request_id,
        vec!["xyz:ETH".to_string()],
        11,
        Ok(HashMap::from([("xyz:ETH".to_string(), context(2.0))]).into()),
    ));

    assert!(!terminal.symbol_search_contexts_loading);
    assert!(!terminal.symbol_search_ctxs.contains_key("BTC"));
    assert_eq!(
        terminal
            .symbol_search_ctxs
            .get("xyz:ETH")
            .map(|ctx| ctx.day_vlm),
        Some(Some(2.0))
    );
}

#[test]
fn stale_symbol_search_context_result_is_ignored_after_current_completion() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;

    let _task = terminal.request_symbol_search_context_refresh(true);
    let request_id = terminal.symbol_search_contexts_request_id;
    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));
    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        11,
        Ok(HashMap::from([("BTC".to_string(), context(2.0))]).into()),
    ));

    assert!(!terminal.symbol_search_contexts_loading);
    assert_eq!(
        terminal
            .symbol_search_ctxs
            .get("BTC")
            .map(|ctx| ctx.day_vlm),
        Some(Some(1.0))
    );
    assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(10));
}

#[test]
fn symbol_search_context_result_keeps_only_requested_symbols() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC"), perp_symbol("ETH")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;

    let _task = terminal.request_symbol_search_context_refresh(true);
    let request_id = terminal.symbol_search_contexts_request_id;
    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([
            ("BTC".to_string(), context(1.0)),
            ("ETH".to_string(), context(2.0)),
        ])
        .into()),
    ));

    assert_eq!(terminal.symbol_search_ctxs.len(), 1);
    assert!(terminal.symbol_search_ctxs.contains_key("BTC"));
    assert!(!terminal.symbol_search_ctxs.contains_key("ETH"));
}

#[test]
fn symbol_search_context_success_clears_stale_omitted_requested_symbol() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
    terminal
        .symbol_search_ctxs
        .insert("BTC".to_string(), context(9.0));
    terminal.symbol_search_contexts_loading = true;
    terminal.symbol_search_contexts_request_id = 7;
    terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        7,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::new().into()),
    ));

    assert!(!terminal.symbol_search_contexts_loading);
    assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(10));
    assert!(!terminal.symbol_search_ctxs.contains_key("BTC"));
}

#[test]
fn symbol_search_partial_context_keeps_omitted_requested_last_known_value() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC"), spot_symbol("@107")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
    terminal
        .symbol_search_ctxs
        .insert("@107".to_string(), context(9.0));
    terminal.symbol_search_contexts_loading = true;
    terminal.symbol_search_contexts_request_id = 7;
    terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string(), "@107".to_string()];

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        7,
        vec!["BTC".to_string(), "@107".to_string()],
        10,
        Ok(crate::api::WatchlistContextsResponse {
            contexts: HashMap::from([("BTC".to_string(), context(1.0))]),
            partial_errors: vec!["spot: HTTP 503".to_string()],
        }),
    ));

    assert_eq!(
        terminal
            .symbol_search_ctxs
            .get("@107")
            .and_then(|ctx| ctx.day_vlm),
        Some(9.0)
    );
}

#[test]
fn symbol_search_context_error_keeps_existing_cache_without_marking_fresh() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;
    terminal.symbol_search_contexts_last_fetch_ms = Some(10);
    terminal
        .symbol_search_ctxs
        .insert("BTC".to_string(), context(9.0));
    terminal.symbol_search_contexts_loading = true;
    terminal.symbol_search_contexts_request_id = 7;
    terminal.symbol_search_contexts_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        7,
        vec!["BTC".to_string()],
        20,
        Err("network".to_string()),
    ));

    assert!(!terminal.symbol_search_contexts_loading);
    assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, Some(10));
    assert_eq!(
        terminal
            .symbol_search_ctxs
            .get("BTC")
            .map(|ctx| ctx.day_vlm),
        Some(Some(9.0))
    );
    assert_eq!(
        terminal
            .symbol_search_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("24h volume refresh failed: network", true))
    );
}

#[test]
fn empty_symbol_search_scope_invalidates_in_flight_context_result() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.symbol_search_sort_mode = SymbolSearchSortMode::Volume24h;

    let _task = terminal.request_symbol_search_context_refresh(true);
    let stale_request_id = terminal.symbol_search_contexts_request_id;
    terminal.exchange_symbols.clear();
    let _task = terminal.request_symbol_search_context_refresh(true);

    assert!(!terminal.symbol_search_contexts_loading);
    assert!(terminal.symbol_search_contexts_request_symbols.is_empty());
    assert!(terminal.symbol_search_ctxs.is_empty());
    assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, None);

    let _task = terminal.update_symbol_search_market(Message::SymbolSearchContextsLoaded(
        stale_request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(terminal.symbol_search_ctxs.is_empty());
    assert_eq!(terminal.symbol_search_contexts_last_fetch_ms, None);
}
