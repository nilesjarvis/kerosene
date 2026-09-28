use super::*;

#[test]
fn reconcile_session_data_symbols_replaces_unsupported_saved_symbol() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "HYPE".to_string();
    terminal.exchange_symbols = vec![
        exchange_symbol("HYPE", "HYPE", MarketType::Perp),
        exchange_symbol("BTC", "BTC", MarketType::Perp),
    ];
    terminal.session_data.insert(
        7,
        SessionDataInstance::new(
            7,
            "NOT_A_MARKET".to_string(),
            SessionDataLookback::FourWeeks,
        ),
    );
    let old_pending = SessionDataRequest {
        id: 7,
        symbol: "NOT_A_MARKET".to_string(),
        lookback: SessionDataLookback::FourWeeks,
        requested_at_ms: 123,
    };
    {
        let instance = terminal.session_data.get_mut(&7).expect("session data");
        instance.loading = true;
        instance.pending_request = Some(old_pending);
    }

    let _task = terminal.reconcile_session_data_symbols();

    let instance = terminal.session_data.get(&7).expect("session data");
    assert_eq!(instance.symbol, "HYPE");
    assert!(instance.loading);
    let request = instance.pending_request.as_ref().expect("pending request");
    assert_eq!(request.symbol, "HYPE");
    assert_ne!(request.requested_at_ms, 123);
    assert!(instance.error.is_none());
}

#[test]
fn resolved_session_data_symbol_key_prefers_exact_key_over_shared_ticker() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        exchange_symbol("@107", "HYPE", MarketType::Spot),
        exchange_symbol("HYPE", "HYPE", MarketType::Perp),
    ];

    assert_eq!(
        terminal.resolved_session_data_symbol_key("HYPE"),
        Some("HYPE")
    );
}

#[test]
fn symbol_resolution_preserves_catalog_precedence_and_empty_catalog_fallback() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.clear();
    for (input, expected) in [
        ("", None),
        ("  ", None),
        (" #1 ", None),
        (" @107 ", Some("@107")),
        (" raw:ABC ", Some("raw:ABC")),
    ] {
        assert_eq!(terminal.resolved_session_data_symbol_key(input), expected);
    }
    terminal.exchange_symbols = vec![
        exchange_symbol("@107", "HYPE", MarketType::Spot),
        exchange_symbol("HYPE", "HYPE", MarketType::Perp),
        exchange_symbol("@2", "SHARED", MarketType::Spot),
        exchange_symbol("dex:SHARED", "SHARED", MarketType::Perp),
        exchange_symbol("@3", "SPOT_ONLY", MarketType::Spot),
        exchange_symbol("BLOCKED", "OUTCOME", MarketType::Outcome),
        exchange_symbol("dex:BLOCKED", "BLOCKED", MarketType::Perp),
    ];
    for (input, expected) in [
        (" HYPE ", Some("HYPE")),
        ("@107", Some("@107")),
        ("SHARED", Some("dex:SHARED")),
        ("SPOT_ONLY", Some("@3")),
        ("BLOCKED", None),
        ("OUTCOME", None),
        ("missing", None),
    ] {
        assert_eq!(terminal.resolved_session_data_symbol_key(input), expected);
        assert_eq!(
            terminal.session_data_symbol_is_supported(input),
            expected.is_some()
        );
    }
    terminal.market_universe = crate::config::MarketUniverseConfig::hip3_dex("dex");
    terminal.active_symbol = "dex:SHARED".to_string();
    assert_eq!(terminal.retained_session_data_symbol(" @107 "), "@107");
    assert_eq!(terminal.visible_session_data_symbol(" @107 "), "dex:SHARED");
}

#[test]
fn reconciliation_preserves_unchanged_history_and_only_schedules_changed_panes() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.session_data.clear();
    terminal.active_symbol = "HYPE".to_string();
    terminal.exchange_symbols = vec![exchange_symbol("HYPE", "HYPE", MarketType::Perp)];
    let original = history_instance(7, "HYPE");
    terminal.session_data.insert(7, original.clone());
    terminal
        .session_data
        .insert(8, history_instance(8, "MISSING"));
    terminal.config_save_due_at = None;
    let _task = terminal.reconcile_session_data_symbols();
    let unchanged = terminal.session_data.get(&7).expect("unchanged pane");
    assert_eq!(unchanged.bars, original.bars);
    assert_eq!(unchanged.pending_request, original.pending_request);
    assert_eq!(unchanged.search_query, "keep search");
    assert!(unchanged.symbol_picker_open);
    assert_eq!(unchanged.error, original.error);
    let changed = terminal.session_data.get(&8).expect("changed pane");
    assert_eq!(changed.symbol, "HYPE");
    assert!(changed.bars.is_empty());
    assert!(changed.search_query.is_empty());
    assert!(!changed.symbol_picker_open);
    assert!(changed.loading);
    assert!(changed.error.is_none());
    assert_eq!(changed.last_fetch_ms, None);
    assert_eq!(
        changed.pending_request.as_ref().expect("refresh").symbol,
        "HYPE"
    );
    assert!(terminal.config_save_due_at.is_some());
    terminal.config_save_due_at = None;
    let pending = changed.pending_request.clone();
    let _task = terminal.reconcile_session_data_symbols();
    assert!(terminal.config_save_due_at.is_none());
    assert_eq!(
        terminal
            .session_data
            .get(&8)
            .expect("changed pane")
            .pending_request,
        pending
    );
}

#[test]
fn symbol_selection_preserves_same_target_and_rejects_before_clearing_history() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        exchange_symbol("HYPE", "HYPE", MarketType::Perp),
        exchange_symbol("@107", "HYPE", MarketType::Spot),
    ];
    let original = history_instance(7, "HYPE");
    terminal.session_data.insert(7, original.clone());
    terminal.config_save_due_at = None;
    let _task = terminal.select_session_data_symbol(7, " HYPE ".to_string());
    let instance = terminal.session_data.get(&7).expect("same pane");
    assert_eq!(instance.pending_request, original.pending_request);
    assert_eq!(instance.bars, original.bars);
    assert_eq!(instance.error, original.error);
    assert!(instance.search_query.is_empty());
    assert!(!instance.symbol_picker_open);
    assert!(terminal.config_save_due_at.is_none());
    let _task = terminal.select_session_data_symbol(7, "#1".to_string());
    let instance = terminal.session_data.get(&7).expect("rejected target");
    assert_eq!(instance.symbol, "HYPE");
    assert_eq!(instance.pending_request, original.pending_request);
    assert_eq!(instance.bars, original.bars);
    assert!(!instance.loading);
    assert!(instance.error.is_some());
    let _task = terminal.select_session_data_symbol(7, " @107 ".to_string());
    let instance = terminal.session_data.get(&7).expect("spot target");
    assert_eq!(instance.symbol, "@107");
    assert!(instance.bars.is_empty());
    assert!(instance.loading);
    assert!(instance.error.is_none());
    assert_eq!(
        instance
            .pending_request
            .as_ref()
            .expect("spot refresh")
            .symbol,
        "@107"
    );
    assert!(terminal.config_save_due_at.is_some());
}
