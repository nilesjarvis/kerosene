use super::*;

#[test]
fn planner_requests_missing_contexts() {
    let contexts = HashMap::from([("BTC".to_string(), context())]);

    let plan = plan_context_refresh(SymbolSearchContextRefreshInput {
        symbols: symbols(),
        force: false,
        now_ms: 500_000,
        sort_mode: SymbolSearchSortMode::Volume24h,
        contexts_loading: false,
        contexts_last_fetch_ms: Some(400_001),
        contexts: &contexts,
    });

    assert_eq!(
        plan,
        Some(SymbolSearchContextRefreshPlan {
            requested_at: 500_000,
            symbols: symbols(),
        })
    );
}

#[test]
fn planner_requests_stale_contexts() {
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
        contexts_last_fetch_ms: Some(199_999),
        contexts: &contexts,
    });

    assert_eq!(plan.map(|candidate| candidate.symbols), Some(symbols()));
}

#[test]
fn planner_force_requests_fresh_complete_contexts() {
    let contexts = HashMap::from([
        ("BTC".to_string(), context()),
        ("ETH".to_string(), context()),
    ]);

    let plan = plan_context_refresh(SymbolSearchContextRefreshInput {
        symbols: symbols(),
        force: true,
        now_ms: 500_000,
        sort_mode: SymbolSearchSortMode::Volume24h,
        contexts_loading: false,
        contexts_last_fetch_ms: Some(400_001),
        contexts: &contexts,
    });

    assert_eq!(plan.map(|candidate| candidate.symbols), Some(symbols()));
}
