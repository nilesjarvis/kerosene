use super::*;

#[test]
fn planner_requests_contexts_for_missing_symbols() {
    let contexts = HashMap::from([("BTC".to_string(), context())]);
    let history_loaded_at =
        HashMap::from([("BTC".to_string(), 90_000), ("ETH".to_string(), 90_000)]);

    let plan = plan(LiveWatchlistRefreshInput {
        symbols: symbols(),
        force: false,
        now_ms: 100_000,
        contexts_last_fetch_ms: Some(90_000),
        contexts: &contexts,
        contexts_loading: false,
        history_loaded_at: &history_loaded_at,
        history_loading: false,
    });

    assert_eq!(plan.context_symbols, symbols());
    assert!(plan.history_symbols.is_empty());
}

#[test]
fn planner_requests_contexts_when_stale_even_if_cached() {
    let contexts = HashMap::from([
        ("BTC".to_string(), context()),
        ("ETH".to_string(), context()),
    ]);
    let history_loaded_at =
        HashMap::from([("BTC".to_string(), 90_000), ("ETH".to_string(), 90_000)]);

    let plan = plan(LiveWatchlistRefreshInput {
        symbols: symbols(),
        force: false,
        now_ms: 100_000,
        contexts_last_fetch_ms: Some(39_999),
        contexts: &contexts,
        contexts_loading: false,
        history_loaded_at: &history_loaded_at,
        history_loading: false,
    });

    assert_eq!(plan.context_symbols, symbols());
    assert!(plan.history_symbols.is_empty());
}

#[test]
fn planner_suppresses_context_requests_while_contexts_are_loading() {
    let contexts = HashMap::new();
    let history_loaded_at = HashMap::new();

    let plan = plan(LiveWatchlistRefreshInput {
        symbols: symbols(),
        force: true,
        now_ms: 100_000,
        contexts_last_fetch_ms: None,
        contexts: &contexts,
        contexts_loading: true,
        history_loaded_at: &history_loaded_at,
        history_loading: true,
    });

    assert!(plan.context_symbols.is_empty());
    assert!(plan.history_symbols.is_empty());
}
