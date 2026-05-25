use super::*;

#[test]
fn planner_requests_only_stale_or_missing_history_symbols() {
    let contexts = HashMap::from([
        ("BTC".to_string(), context()),
        ("ETH".to_string(), context()),
    ]);
    let history_loaded_at = HashMap::from([("BTC".to_string(), 90_000)]);

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

    assert!(plan.context_symbols.is_empty());
    assert_eq!(plan.history_symbols, vec!["ETH".to_string()]);
}
