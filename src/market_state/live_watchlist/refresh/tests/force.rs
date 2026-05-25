use super::*;

#[test]
fn planner_returns_no_requests_for_empty_symbols() {
    let contexts = HashMap::new();
    let history_loaded_at = HashMap::new();

    let plan = plan(LiveWatchlistRefreshInput {
        symbols: Vec::new(),
        force: true,
        now_ms: 100_000,
        contexts_last_fetch_ms: None,
        contexts: &contexts,
        contexts_loading: false,
        history_loaded_at: &history_loaded_at,
        history_loading: false,
    });

    assert!(!plan.has_requests());
}

#[test]
fn planner_force_requests_all_non_loading_data() {
    let contexts = HashMap::from([
        ("BTC".to_string(), context()),
        ("ETH".to_string(), context()),
    ]);
    let history_loaded_at =
        HashMap::from([("BTC".to_string(), 90_000), ("ETH".to_string(), 90_000)]);

    let plan = plan(LiveWatchlistRefreshInput {
        symbols: symbols(),
        force: true,
        now_ms: 100_000,
        contexts_last_fetch_ms: Some(90_000),
        contexts: &contexts,
        contexts_loading: false,
        history_loaded_at: &history_loaded_at,
        history_loading: false,
    });

    assert_eq!(plan.context_symbols, symbols());
    assert_eq!(plan.history_symbols, symbols());
}
