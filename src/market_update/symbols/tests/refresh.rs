use super::*;

#[test]
fn outcome_label_recording_preserves_expired_names_and_only_saves_changes() {
    let mut terminal = TradingTerminal::boot().0;
    let mut outcome = outcome_symbol("#950");
    let label = TradingTerminal::exchange_symbol_display_name(&outcome);
    terminal.outcome_display_labels = HashMap::from([
        ("#950".into(), label),
        ("#old".into(), "Expired outcome".into()),
    ]);
    terminal.exchange_symbols = vec![perp_symbol("BTC"), outcome.clone()];
    terminal.config_save_due_at = None;
    terminal.record_outcome_display_labels();
    assert!(terminal.config_save_due_at.is_none());

    outcome
        .outcome
        .as_mut()
        .expect("outcome metadata")
        .question_name = Some("Updated question".into());
    let updated_label = TradingTerminal::exchange_symbol_display_name(&outcome);
    terminal.exchange_symbols = vec![perp_symbol("BTC"), outcome];
    terminal.record_outcome_display_labels();
    assert!(terminal.config_save_due_at.is_some());
    assert_eq!(terminal.outcome_display_labels["#950"], updated_label);
    assert_eq!(terminal.outcome_display_labels["#old"], "Expired outcome");
    assert!(!terminal.outcome_display_labels.contains_key("BTC"));

    terminal.exchange_symbols.clear();
    terminal.config_save_due_at = None;
    terminal.record_outcome_display_labels();
    assert_eq!(terminal.outcome_display_labels.len(), 2);
    assert!(terminal.config_save_due_at.is_none());
}

#[test]
fn failed_spot_metadata_retains_markets_but_disables_orders_until_verified() {
    let mut terminal = TradingTerminal::boot().0;
    let spot = spot_symbol("@107");
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("HYPE"), spot.clone()])));
    assert!(!terminal.spot_metadata_degraded);
    assert!(
        terminal
            .validate_exchange_symbol_orderable(&spot, "Active")
            .is_ok()
    );

    let _task = terminal.apply_symbols_loaded(Ok(ExchangeSymbolsPayload {
        symbols: vec![perp_symbol("HYPE")],
        perp_dexes: None,
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: true,
        outcome_meta_failed: false,
    }));

    assert!(terminal.spot_metadata_degraded);
    let retained = terminal
        .exchange_symbol_for_key("@107")
        .expect("last-known spot symbol retained");
    let error = terminal
        .validate_exchange_symbol_orderable(retained, "Active")
        .expect_err("unverified spot metadata must fail closed");
    assert!(error.contains("temporarily unverified"));
    assert!(terminal.symbol_search_status.as_ref().is_some_and(
        |(message, is_error)| *is_error && message.contains("spot trading is disabled")
    ));

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("HYPE"), spot])));

    assert!(!terminal.spot_metadata_degraded);
    let verified = terminal
        .exchange_symbol_for_key("@107")
        .expect("fresh spot symbol");
    assert!(
        terminal
            .validate_exchange_symbol_orderable(verified, "Active")
            .is_ok()
    );
    assert!(
        terminal
            .symbol_search_status
            .as_ref()
            .is_some_and(|(message, is_error)| !*is_error && message.contains("available again"))
    );
}

#[test]
fn cached_spot_metadata_is_visible_but_requires_immediate_live_verification() {
    let mut terminal = TradingTerminal::boot().0;
    let spot = spot_symbol("@107");
    let mut cached = payload(vec![perp_symbol("HYPE"), spot.clone()]);
    cached.loaded_from_cache = true;

    let task = terminal.apply_symbols_loaded(Ok(cached));

    assert!(terminal.exchange_symbol_for_key("@107").is_some());
    assert!(terminal.spot_metadata_degraded);
    assert!(terminal.exchange_symbols_refresh_inflight);
    assert!(task.units() >= 1, "live verification must be scheduled now");
    let cached_spot = terminal
        .exchange_symbol_for_key("@107")
        .expect("cached spot remains visible");
    let error = terminal
        .validate_exchange_symbol_orderable(cached_spot, "Active")
        .expect_err("cache provenance cannot authorize a spot order");
    assert!(error.contains("temporarily unverified"));
    assert!(terminal.symbol_search_status.as_ref().is_some_and(
        |(message, is_error)| *is_error && message.contains("live metadata is verified")
    ));

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("HYPE"), spot])));

    assert!(!terminal.spot_metadata_degraded);
    assert!(!terminal.exchange_symbols_refresh_inflight);
    let verified_spot = terminal
        .exchange_symbol_for_key("@107")
        .expect("live-verified spot");
    assert!(
        terminal
            .validate_exchange_symbol_orderable(verified_spot, "Active")
            .is_ok()
    );
}

#[test]
fn cached_and_failed_outcome_metadata_remains_inspectable_until_live_recovery() {
    let mut terminal = TradingTerminal::boot().0;
    let live = outcome_symbol("#950");
    let mut cached = payload(vec![live.clone()]);
    cached.loaded_from_cache = true;
    let task = terminal.apply_symbols_loaded(Ok(cached));
    assert!(task.units() >= 1);
    assert!(terminal.exchange_symbols_refresh_inflight);
    assert!(terminal.exchange_symbols[0].is_user_selectable_market());
    assert!(!terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![live.clone()])));
    assert!(terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
    let _task = terminal.apply_symbols_loaded(Err("offline".into()));
    assert!(terminal.exchange_symbols[0].is_user_selectable_market());
    assert!(!terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![live])));
    assert!(terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
}

#[test]
fn cold_partial_load_preserves_saved_spot_selection() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "@107".to_string();
    terminal.active_symbol_display = "HYPE/USDC".to_string();

    let _task = terminal.apply_symbols_loaded(Ok(ExchangeSymbolsPayload {
        symbols: vec![perp_symbol("HYPE")],
        perp_dexes: None,
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: true,
        outcome_meta_failed: false,
    }));

    assert_eq!(terminal.active_symbol, "@107");
    assert_eq!(terminal.active_symbol_display, "HYPE/USDC");
    assert!(terminal.spot_metadata_degraded);
}

#[test]
fn perp_failure_does_not_discard_fresh_spot_metadata() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC")];

    let merged = terminal.merge_symbols_payload(ExchangeSymbolsPayload {
        symbols: vec![spot_symbol("@107")],
        perp_dexes: None,
        loaded_from_cache: false,
        perp_meta_failed: true,
        spot_meta_failed: false,
        outcome_meta_failed: false,
    });

    assert!(
        merged
            .iter()
            .any(|symbol| symbol.market_type == MarketType::Perp && symbol.key == "BTC")
    );
    assert!(
        merged
            .iter()
            .any(|symbol| symbol.market_type == MarketType::Spot && symbol.key == "@107")
    );
}

#[test]
fn symbols_refresh_preserves_open_widgets_when_market_universe_is_unchanged() {
    let mut changed_btc = perp_symbol("BTC");
    changed_btc.max_leverage = 20;
    for (before, after) in [
        (Vec::new(), vec![perp_symbol("BTC")]),
        (vec![perp_symbol("BTC")], vec![changed_btc]),
        (
            vec![perp_symbol("BTC")],
            vec![perp_symbol("BTC"), outcome_symbol("#950")],
        ),
        (
            vec![perp_symbol("BTC"), outcome_symbol("#950")],
            vec![perp_symbol("BTC")],
        ),
    ] {
        let mut baseline = symbols_refresh_terminal(false);
        baseline.exchange_symbols = before.clone();
        let baseline_task = baseline.apply_symbols_loaded(Ok(payload(after.clone())));

        let mut terminal = symbols_refresh_terminal(true);
        terminal.exchange_symbols = before;
        let task = terminal.apply_symbols_loaded(Ok(payload(after.clone())));

        assert_eq!(terminal.exchange_symbols, after);
        assert_eq!(terminal.market_universe, MarketUniverseConfig::All);
        // ChartReload and SpaghettiReload are deferred messages: checking
        // candles alone would miss the reset that happens on the next update.
        assert_eq!(task.units(), baseline_task.units());
        assert_eq!(terminal.charts[&7].chart.candles[0].close, 100.0);
        assert!(terminal.charts[&7].candle_fetch_request.is_none());
        let series = &terminal.spaghetti_charts[&8].canvas.series[0];
        assert!(series.loaded);
        assert_eq!(series.candles[0].close, 100.0);
        assert!(!terminal.order_books[&9].book_loading);
        assert!(terminal.order_books[&9].pending_book_request_id().is_none());
    }
}

#[test]
fn symbols_refresh_still_reloads_widgets_when_selected_market_universe_disappears() {
    let mut baseline = symbols_refresh_terminal(false);
    baseline.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    baseline.exchange_symbols = vec![perp_symbol("xyz:BTC")];
    let baseline_task = baseline.apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC")])));

    let mut terminal = symbols_refresh_terminal(true);
    terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    terminal.exchange_symbols = vec![perp_symbol("xyz:BTC")];
    let task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC")])));

    assert_eq!(terminal.market_universe, MarketUniverseConfig::All);
    // The widened filter restores the chart, comparison chart, and book.
    assert_eq!(task.units(), baseline_task.units() + 3);
    assert!(terminal.order_books[&9].book_loading);
    assert!(terminal.order_books[&9].pending_book_request_id().is_some());
}

#[test]
fn symbols_refresh_keeps_registered_dex_selected_after_last_market_delists() {
    let mut terminal = symbols_refresh_terminal(false);
    terminal.market_universe = MarketUniverseConfig::hip3_dex("inactive");
    terminal.exchange_symbols = vec![perp_symbol("BTC"), perp_symbol("inactive:ABC")];
    let mut refreshed = payload(vec![perp_symbol("BTC")]);
    refreshed.perp_dexes = Some(vec![crate::api::PerpDex {
        name: "inactive".to_string(),
        collateral_token: Some(404),
    }]);

    let _task = terminal.apply_symbols_loaded(Ok(refreshed.clone()));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );
    assert_eq!(terminal.visible_collateral_token(), Some(404));
    assert!(terminal.exchange_symbol_for_key("inactive:ABC").is_none());

    // An unchanged background refresh must also preserve the selection.
    let _task = terminal.apply_symbols_loaded(Ok(refreshed));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );

    let mut failed = payload(vec![spot_symbol("@107")]);
    failed.perp_meta_failed = true;
    let _task = terminal.apply_symbols_loaded(Ok(failed));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );
    assert_eq!(terminal.perp_dexes[0].name, "inactive");
    let _task = terminal.apply_symbols_loaded(Err("metadata unavailable".to_string()));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );
    assert_eq!(terminal.perp_dexes[0].name, "inactive");
}

#[test]
fn legacy_cached_symbols_preserve_inactive_selection_until_live_registry_arrives() {
    let mut terminal = symbols_refresh_terminal(false);
    terminal.market_universe = MarketUniverseConfig::hip3_dex("inactive");
    let mut cached = payload(vec![perp_symbol("BTC")]);
    cached.loaded_from_cache = true;
    let _task = terminal.apply_symbols_loaded(Ok(cached));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );

    let mut live = payload(vec![perp_symbol("BTC")]);
    live.perp_dexes = Some(vec![crate::api::PerpDex {
        name: "inactive".to_string(),
        collateral_token: Some(404),
    }]);
    let _task = terminal.apply_symbols_loaded(Ok(live));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );
    assert_eq!(terminal.visible_collateral_token(), Some(404));
}

#[test]
fn registry_only_refresh_adds_and_removes_dexes_even_when_symbols_are_unchanged() {
    let mut terminal = symbols_refresh_terminal(false);
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    let mut live = payload(terminal.exchange_symbols.clone());
    live.perp_dexes = Some(vec![crate::api::PerpDex {
        name: "empty".to_string(),
        collateral_token: Some(404),
    }]);
    let _task = terminal.apply_symbols_loaded(Ok(live.clone()));
    let empty = MarketUniverseConfig::hip3_dex("empty");
    assert!(terminal.market_universe_options().contains(&empty));
    assert!(
        terminal
            .account_data_fetch_scope()
            .hip3_dexes(&[])
            .contains(&"empty")
    );

    terminal.market_universe = empty.clone();
    live.perp_dexes = Some(Vec::new());
    let _task = terminal.apply_symbols_loaded(Ok(live));
    assert!(!terminal.market_universe_options().contains(&empty));
    assert_eq!(terminal.market_universe, MarketUniverseConfig::All);
}

#[test]
fn unchanged_live_metadata_normalizes_selection_preserved_during_cache_load() {
    let mut terminal = symbols_refresh_terminal(false);
    terminal.market_universe = MarketUniverseConfig::hip3_dex("removed");
    let mut live = payload(vec![perp_symbol("BTC")]);
    live.perp_dexes = Some(Vec::new());
    let mut cached = live.clone();
    cached.loaded_from_cache = true;
    let _task = terminal.apply_symbols_loaded(Ok(cached));
    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("removed")
    );

    let _task = terminal.apply_symbols_loaded(Ok(live));
    assert_eq!(terminal.market_universe, MarketUniverseConfig::All);
}

#[test]
fn symbols_loaded_keeps_outcome_labels_when_outcome_meta_fails_but_rejects_orderability() {
    let mut terminal = TradingTerminal::boot().0;
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));
    assert_eq!(terminal.exchange_symbols.len(), 1);

    let _task = terminal.apply_symbols_loaded(Ok(ExchangeSymbolsPayload {
        symbols: Vec::new(),
        perp_dexes: None,
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: false,
        outcome_meta_failed: true,
    }));

    assert_eq!(
        terminal.exchange_symbols.len(),
        1,
        "previously loaded outcome symbols must survive a failed outcomeMeta refresh"
    );
    assert_eq!(terminal.exchange_symbols[0].key, "#950");
    assert_eq!(
        terminal.exchange_symbols[0].display_name.as_deref(),
        Some("YES: Will BTC close green?")
    );
    assert!(
        !terminal.exchange_symbols[0]
            .outcome
            .as_ref()
            .expect("terms remain available")
            .contract
            .verified
    );
    assert!(terminal.exchange_symbols[0].is_user_selectable_market());
    assert!(!terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
    assert_eq!(
        terminal.display_name_for_symbol("#950"),
        "YES: Will BTC close green?"
    );
    assert_eq!(
        terminal
            .symbol_search_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some((
            "Outcome market metadata failed to load; retrying shortly",
            true
        ))
    );
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));
    assert!(terminal.exchange_symbol_is_orderable(&terminal.exchange_symbols[0]));
}

#[test]
fn symbols_loaded_records_outcome_display_labels() {
    let mut terminal = TradingTerminal::boot().0;
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));

    assert_eq!(
        terminal
            .outcome_display_labels
            .get("#950")
            .map(String::as_str),
        Some("YES: Will BTC close green?")
    );

    // The cached label keeps resolving the coin after the market expires
    // and disappears from outcomeMeta.
    let _task = terminal.apply_symbols_loaded(Ok(payload(Vec::new())));
    assert_eq!(
        terminal.display_name_for_symbol("#950"),
        "YES: Will BTC close green?"
    );
}

#[test]
fn symbols_loaded_refreshes_spaghetti_series_displays() {
    let mut terminal = TradingTerminal::boot().0;
    let mut inst = crate::spaghetti_state::SpaghettiChartInstance::new_empty(3);
    inst.canvas.series.push(crate::spaghetti::Series {
        symbol: "#950".to_string(),
        display: "#950".to_string(),
        candles: Vec::new(),
        color: iced::Color::WHITE,
        loaded: false,
    });
    terminal.spaghetti_charts.insert(3, inst);

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));

    let inst = terminal.spaghetti_charts.get(&3).expect("spaghetti chart");
    assert_eq!(inst.canvas.series[0].display, "YES: Will BTC close green?");
}

#[test]
fn symbols_load_error_after_successful_load_keeps_symbols_and_stays_quiet() {
    let mut terminal = TradingTerminal::boot().0;
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));
    terminal.symbol_search_status = None;

    let _task = terminal.apply_symbols_loaded(Err("network down".to_string()));

    assert_eq!(terminal.exchange_symbols.len(), 1);
    assert!(
        terminal.symbol_search_status.is_none(),
        "background refresh failures must not surface error status"
    );
    assert!(!terminal.symbols_loading);
}

#[test]
fn symbols_load_error_before_initial_load_redacts_status_and_toast() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.clear();
    terminal.toasts.clear();

    let _task = terminal.apply_symbols_loaded(Err(
        "symbol fetch failed: api_key=key-secret auth_token=token-secret".to_string(),
    ));

    let status = terminal.symbol_search_status.as_ref().expect("status");
    assert!(status.1);
    assert!(status.0.contains("api_key=<redacted>"));
    assert!(status.0.contains("auth_token=<redacted>"));
    assert!(!status.0.contains("key-secret"));
    assert!(!status.0.contains("token-secret"));

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert_eq!(toast.message, status.0);
}
