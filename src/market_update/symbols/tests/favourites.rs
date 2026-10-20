use super::*;

fn terminal_with_favourites(symbols: &[&str], tape_enabled: bool) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot_from_config(crate::config::KeroseneConfig {
        active_symbol: "BTC".into(),
        favourite_symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
        ticker_tape_enabled: tape_enabled,
        ..Default::default()
    });
    terminal.config_save_due_at = None;
    terminal
}

#[test]
fn live_metadata_removes_delisted_favourites_and_persists_saved_layouts() {
    for tape_enabled in [false, true] {
        let mut terminal = terminal_with_favourites(
            &["xyz:OLD", "ETH", "OLD", "BTC", "@999", "#9990"],
            tape_enabled,
        );
        terminal.saved_layouts = vec![terminal.saved_layout_snapshot("Saved".into())];
        terminal.exchange_symbols = vec![
            perp_symbol("xyz:OLD"),
            perp_symbol("OLD"),
            perp_symbol("BTC"),
            perp_symbol("ETH"),
            spot_symbol("@999"),
            outcome_symbol("#9990"),
        ];
        terminal.ticker_tape_ctxs = HashMap::from([
            ("xyz:OLD".into(), context(1.0)),
            ("BTC".into(), context(2.0)),
        ]);

        let _task = terminal
            .apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC"), perp_symbol("ETH")])));

        assert_eq!(terminal.favourite_symbols, ["ETH", "BTC"]);
        assert_eq!(terminal.saved_layouts[0].favourite_symbols, ["ETH", "BTC"]);
        assert_eq!(terminal.favourite_symbols_config_values(), ["ETH", "BTC"]);
        assert!(terminal.config_save_due_at.is_some());
        assert!(!terminal.ticker_tape_ctxs.contains_key("xyz:OLD"));
        assert!(terminal.ticker_tape_ctxs.contains_key("BTC"));

        let layout = terminal.saved_layouts[0].clone();
        let _task = terminal.apply_layout(layout);
        assert_eq!(terminal.favourite_symbols, ["ETH", "BTC"]);
    }
}

#[test]
fn cached_metadata_preserves_favourites_until_unchanged_live_verification() {
    let mut terminal = terminal_with_favourites(&["xyz:OLD", "BTC"], true);
    let live = payload(vec![perp_symbol("BTC")]);
    let mut cached = live.clone();
    cached.loaded_from_cache = true;

    let _task = terminal.apply_symbols_loaded(Ok(cached));
    assert_eq!(terminal.favourite_symbols, ["xyz:OLD", "BTC"]);
    assert!(terminal.config_save_due_at.is_none());

    let _task = terminal.apply_symbols_loaded(Ok(live));
    assert_eq!(terminal.favourite_symbols, ["BTC"]);
    assert!(terminal.config_save_due_at.is_some());
    assert!(terminal.ticker_tape_contexts_refresh_pending);

    terminal.config_save_due_at = None;
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC")])));
    assert!(terminal.config_save_due_at.is_none());
}

#[test]
fn partial_metadata_only_prunes_favourites_from_successful_families() {
    for failed_family in [MarketType::Perp, MarketType::Spot, MarketType::Outcome] {
        for warm in [false, true] {
            let mut terminal =
                terminal_with_favourites(&["xyz:OLD", "OLD", "@999", "OLD/USDC", "#9990"], false);
            if warm {
                terminal.exchange_symbols = vec![
                    perp_symbol("xyz:OLD"),
                    perp_symbol("OLD"),
                    spot_symbol("@999"),
                    spot_symbol("OLD/USDC"),
                    outcome_symbol("#9990"),
                ];
            }
            let mut refreshed = payload(vec![perp_symbol("BTC")]);
            refreshed.perp_meta_failed = failed_family == MarketType::Perp;
            refreshed.spot_meta_failed = failed_family == MarketType::Spot;
            refreshed.outcome_meta_failed = failed_family == MarketType::Outcome;

            let _task = terminal.apply_symbols_loaded(Ok(refreshed));

            let expected = match failed_family {
                MarketType::Perp => vec!["xyz:OLD", "OLD"],
                MarketType::Spot => vec!["@999", "OLD/USDC"],
                MarketType::Outcome => vec!["#9990"],
            };
            assert_eq!(terminal.favourite_symbols, expected);
            assert_eq!(terminal.saved_layouts[0].favourite_symbols, expected);
        }
    }
}

#[test]
fn failed_metadata_preserves_all_favourites_and_contexts() {
    let mut terminal = terminal_with_favourites(&["xyz:OLD", "BTC", "@999", "#9990"], false);
    terminal
        .ticker_tape_ctxs
        .insert("xyz:OLD".into(), context(1.0));
    let favourites = terminal.favourite_symbols.clone();

    let _task = terminal.apply_symbols_loaded(Err("offline".into()));
    assert_eq!(terminal.favourite_symbols, favourites);
    assert!(terminal.config_save_due_at.is_none());

    let mut failed = payload(Vec::new());
    failed.perp_meta_failed = true;
    failed.spot_meta_failed = true;
    failed.outcome_meta_failed = true;
    let _task = terminal.apply_symbols_loaded(Ok(failed));
    assert_eq!(terminal.favourite_symbols, favourites);
    assert!(terminal.ticker_tape_ctxs.contains_key("xyz:OLD"));
    assert!(terminal.config_save_due_at.is_none());
}

#[test]
fn favourite_cleanup_uses_exact_market_keys_and_preserves_spot_aliases_and_filters() {
    let mut terminal =
        terminal_with_favourites(&["BTC", "old:BTC", "xyz:BTC", "ETH", "@0", "@107"], false);
    terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    terminal.symbol_search_query = "no matches".into();
    terminal.muted_tickers.insert("ETH".into());
    let mut hip3 = perp_symbol("xyz:BTC");
    hip3.ticker = "BTC".into();

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![
        hip3,
        perp_symbol("ETH"),
        canonical_purr_symbol(),
        spot_symbol("@107"),
    ])));

    assert_eq!(terminal.favourite_symbols, ["xyz:BTC", "ETH", "@0", "@107"]);
}

#[test]
fn late_ticker_context_cannot_restore_delisted_favourite() {
    let mut terminal = terminal_with_favourites(&["BTC", "xyz:OLD"], true);
    let request_id = terminal.ticker_tape_contexts_request_id;
    let requested_symbols = terminal.ticker_tape_contexts_request_symbols.clone();
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC")])));

    let _task = terminal.update_market(Message::TickerTapeContextsLoaded(
        request_id,
        requested_symbols,
        10,
        Ok(HashMap::from([
            ("xyz:OLD".into(), context(1.0)),
            ("BTC".into(), context(2.0)),
        ])
        .into()),
    ));

    assert_eq!(terminal.favourite_symbols, ["BTC"]);
    assert!(!terminal.ticker_tape_ctxs.contains_key("xyz:OLD"));
    assert!(terminal.ticker_tape_ctxs.contains_key("BTC"));
    assert_eq!(terminal.ticker_tape_contexts_request_symbols, ["BTC"]);
}

#[test]
fn removing_last_favourite_invalidates_pending_ticker_contexts() {
    let mut terminal = terminal_with_favourites(&["xyz:OLD"], true);
    let request_id = terminal.ticker_tape_contexts_request_id;
    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC")])));

    assert!(terminal.favourite_symbols.is_empty());
    assert!(!terminal.ticker_tape_contexts_loading);
    assert!(terminal.ticker_tape_contexts_request_symbols.is_empty());

    let _task = terminal.update_market(Message::TickerTapeContextsLoaded(
        request_id,
        vec!["xyz:OLD".into()],
        10,
        Ok(HashMap::from([("xyz:OLD".into(), context(1.0))]).into()),
    ));
    assert!(terminal.ticker_tape_ctxs.is_empty());
}

#[test]
fn cleanup_persists_delisted_favourites_that_only_exist_in_saved_layouts() {
    let mut terminal = terminal_with_favourites(&["BTC"], false);
    terminal.saved_layouts[0]
        .favourite_symbols
        .push("xyz:OLD".into());
    terminal.exchange_symbols = vec![perp_symbol("BTC")];

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![perp_symbol("BTC")])));

    assert_eq!(terminal.favourite_symbols, ["BTC"]);
    assert_eq!(terminal.saved_layouts[0].favourite_symbols, ["BTC"]);
    assert!(terminal.config_save_due_at.is_some());
}
