use super::*;

#[test]
fn planner_skips_when_sort_mode_does_not_need_volume() {
    let contexts = HashMap::new();

    let plan = plan_context_refresh(SymbolSearchContextRefreshInput {
        symbols: symbols(),
        force: true,
        now_ms: 500_000,
        sort_mode: SymbolSearchSortMode::Relevance,
        contexts_loading: false,
        contexts_last_fetch_ms: None,
        contexts: &contexts,
    });

    assert_eq!(plan, None);
}

#[test]
fn planner_skips_empty_symbols_or_loading_state() {
    let contexts = HashMap::new();

    let empty = plan_context_refresh(SymbolSearchContextRefreshInput {
        symbols: Vec::new(),
        force: true,
        now_ms: 500_000,
        sort_mode: SymbolSearchSortMode::Volume24h,
        contexts_loading: false,
        contexts_last_fetch_ms: None,
        contexts: &contexts,
    });
    let loading = plan_context_refresh(SymbolSearchContextRefreshInput {
        symbols: symbols(),
        force: true,
        now_ms: 500_000,
        sort_mode: SymbolSearchSortMode::Volume24h,
        contexts_loading: true,
        contexts_last_fetch_ms: None,
        contexts: &contexts,
    });

    assert_eq!(empty, None);
    assert_eq!(loading, None);
}

#[test]
fn planner_skips_fresh_complete_contexts_without_force() {
    let contexts = HashMap::from([
        ("BTC".to_string(), context()),
        ("ETH".to_string(), context()),
    ]);

    let plan = plan_context_refresh(SymbolSearchContextRefreshInput {
        symbols: symbols(),
        force: false,
        now_ms: 500_000,
        sort_mode: SymbolSearchSortMode::Volume24h,
        contexts_loading: false,
        contexts_last_fetch_ms: Some(400_001),
        contexts: &contexts,
    });

    assert_eq!(plan, None);
}
