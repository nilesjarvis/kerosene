use crate::api::WatchlistContext;
use crate::api::{MarketType, OutcomeSymbolInfo};

use super::*;

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        growth_mode: false,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

fn watchlist(symbols: &[&str]) -> LiveWatchlistInstance {
    LiveWatchlistInstance {
        id: 1,
        preset_id: None,
        symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
        search_query: String::new(),
        sort_column: config::LiveWatchlistSortColumn::Symbol,
        sort_direction: config::SortDirection::Ascending,
        visible_columns: Vec::new(),
        row_cache: Vec::new(),
    }
}

fn refresh_rows(terminal: &mut TradingTerminal, symbols: &[&str]) -> Vec<LiveWatchlistRowData> {
    terminal.live_watchlists.insert(1, watchlist(symbols));
    terminal.refresh_live_watchlist_row_cache(1);
    terminal.live_watchlists[&1].row_cache.clone()
}

fn perp_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "perp".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 10,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn row(symbol: &str, display: &str, mid_px: Option<f64>) -> LiveWatchlistRowData {
    LiveWatchlistRowData {
        sym_key: symbol.to_string(),
        display: display.to_string(),
        mid_px,
        pct_5m: None,
        pct_30m: None,
        pct_1h: None,
        pct_24h: None,
        funding: None,
    }
}

fn context(prev_day_px: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: Some(prev_day_px),
        mark_px: None,
        day_vlm: None,
        open_interest_notional: None,
    }
}

#[test]
fn watchlist_rows_resolve_outcome_display_through_canonical_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));

    let rows = refresh_rows(&mut terminal, &["#950"]);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].display, "YES: Will BTC close green?");
}

#[test]
fn watchlist_rows_fall_back_to_cached_outcome_label_for_unloaded_keys() {
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .outcome_display_labels
        .insert("#950".to_string(), "YES: Will BTC close green?".to_string());

    let rows = refresh_rows(&mut terminal, &["#950"]);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].display, "YES: Will BTC close green?");
}

#[test]
fn watchlist_percent_change_requires_current_and_previous_prices() {
    assert_eq!(percent_change(Some(110.0), Some(100.0)), Some(10.0));
    assert_eq!(percent_change(None, Some(100.0)), None);
    assert_eq!(percent_change(Some(110.0), None), None);
    assert_eq!(percent_change(Some(110.0), Some(0.0)), None);
    assert_eq!(percent_change(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(f64::INFINITY)), None);
}

#[test]
fn watchlist_rows_do_not_borrow_native_context_for_prefixed_hip3_symbol() {
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .exchange_symbols
        .push(perp_symbol("xyz:NVDA", "NVDA"));
    terminal.all_mids.insert("xyz:NVDA".to_string(), 110.0);
    terminal
        .all_mids_updated_at_ms
        .insert("xyz:NVDA".to_string(), crate::ws::now_ms());
    terminal
        .live_watchlist_ctxs
        .insert("NVDA".to_string(), context(100.0));

    let rows = refresh_rows(&mut terminal, &["xyz:NVDA"]);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pct_24h, None);
}

#[test]
fn watchlist_price_sort_puts_missing_prices_last_in_ascending_order() {
    assert_eq!(sortable_cmp(Some(10.0), Some(20.0), false), Ordering::Less);
    assert_eq!(
        sortable_cmp(Some(10.0), Some(20.0), true),
        Ordering::Greater
    );
    assert_eq!(sortable_cmp(Some(10.0), None, true), Ordering::Less);
    assert_eq!(sortable_cmp(None, Some(10.0), true), Ordering::Greater);
}

#[test]
fn sorted_rows_use_requested_column_and_direction() {
    let rows = vec![
        row("BTC", "Bitcoin", Some(10.0)),
        row("ETH", "Ethereum", None),
        row("SOL", "Solana", Some(20.0)),
    ];

    let sorted = sort_live_watchlist_rows(
        rows,
        config::LiveWatchlistSortColumn::Price,
        config::SortDirection::Descending,
    );

    assert_eq!(
        sorted
            .iter()
            .map(|row| row.sym_key.as_str())
            .collect::<Vec<_>>(),
        vec!["SOL", "BTC", "ETH"]
    );
}

#[test]
fn bulk_and_single_refresh_preserve_market_rows_and_pane_sorting() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.live_watchlists.clear();
    let mut duplicate = perp_symbol("BTC", "BTC");
    duplicate.display_name = Some("Bitcoin".to_string());
    let mut incomplete = outcome_symbol("#970");
    incomplete.outcome = None;
    let mut fallback = outcome_symbol("#980");
    fallback
        .outcome
        .as_mut()
        .expect("fixture has outcome metadata")
        .is_question_fallback = true;
    terminal.exchange_symbols = vec![
        perp_symbol("BTC", "BTC"),
        duplicate,
        outcome_symbol("#950"),
        incomplete,
        fallback,
    ];
    terminal.muted_tickers.insert("MUTED".to_string());
    terminal
        .outcome_display_labels
        .insert("#960".to_string(), "Cached outcome".to_string());
    for (key, price) in [("BTC", 200.0), ("#950", 0.5)] {
        terminal.all_mids.insert(key.to_string(), price);
        terminal
            .all_mids_updated_at_ms
            .insert(key.to_string(), crate::ws::now_ms());
    }
    let mut btc_context = context(400.0);
    btc_context.funding = Some(-0.01);
    terminal
        .live_watchlist_ctxs
        .insert("BTC".to_string(), btc_context);
    terminal
        .live_watchlist_ctxs
        .insert("#950".to_string(), context(1.0));
    terminal
        .live_watchlist_history
        .insert("BTC".to_string(), (100.0, 50.0, 0.0));

    let mut first = watchlist(&[
        "MISSING", "BTC", "#950", "BTC", "#960", "MUTED", "#970", "#980",
    ]);
    first.sort_column = config::LiveWatchlistSortColumn::Price;
    first.sort_direction = config::SortDirection::Descending;
    let mut second = watchlist(&["BTC", "#960"]);
    second.id = 2;
    second.sort_direction = config::SortDirection::Descending;
    let mut empty = watchlist(&[]);
    empty.id = 3;
    terminal.live_watchlists = [(1, first), (2, second), (3, empty)].into();

    let btc = LiveWatchlistRowData {
        pct_5m: Some(100.0),
        pct_30m: Some(300.0),
        pct_24h: Some(-50.0),
        funding: Some(-0.01),
        ..row("BTC", "Bitcoin", Some(200.0))
    };
    let outcome = LiveWatchlistRowData {
        pct_24h: Some(-50.0),
        ..row("#950", "YES: Will BTC close green?", Some(0.5))
    };
    let cached = row("#960", "Cached outcome", None);
    let expected_first = vec![
        btc.clone(),
        btc.clone(),
        outcome,
        row("MISSING", "MISSING", None),
        cached.clone(),
    ];
    let expected_second = vec![cached, btc];
    for bulk in [true, false] {
        for watchlist in terminal.live_watchlists.values_mut() {
            watchlist.row_cache = vec![row("STALE", "Stale", None)];
        }
        if bulk {
            terminal.refresh_live_watchlist_row_caches();
        } else {
            for id in [1, 2, 3] {
                terminal.refresh_live_watchlist_row_cache(id);
            }
        }
        assert_eq!(terminal.live_watchlists[&1].row_cache, expected_first);
        assert_eq!(terminal.live_watchlists[&2].row_cache, expected_second);
        assert!(terminal.live_watchlists[&3].row_cache.is_empty());
    }
}

#[test]
fn refresh_uses_current_metadata_and_visibility_only_for_selected_panes() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol("BTC", "Original")];
    let first = watchlist(&["BTC"]);
    let mut second = first.clone();
    second.id = 2;
    terminal.live_watchlists = [(1, first), (2, second)].into();
    terminal.refresh_live_watchlist_row_caches();
    let original = vec![row("BTC", "Original", None)];
    assert_eq!(terminal.live_watchlists[&1].row_cache, original);
    assert_eq!(terminal.live_watchlists[&2].row_cache, original);

    terminal.exchange_symbols = vec![perp_symbol("BTC", "Changed")];
    terminal.refresh_live_watchlist_row_cache(1);
    terminal.refresh_live_watchlist_row_cache(99);
    assert_eq!(terminal.live_watchlists.len(), 2);
    assert_eq!(
        terminal.live_watchlists[&1].row_cache,
        vec![row("BTC", "Changed", None)]
    );
    assert_eq!(terminal.live_watchlists[&2].row_cache, original);

    terminal.muted_tickers.insert("CHANGED".to_string());
    terminal.refresh_live_watchlist_row_caches();
    assert!(
        terminal
            .live_watchlists
            .values()
            .all(|watchlist| watchlist.row_cache.is_empty())
    );
    terminal.muted_tickers.clear();
    terminal.refresh_live_watchlist_row_caches();
    assert!(
        terminal
            .live_watchlists
            .values()
            .all(|watchlist| watchlist.row_cache.len() == 1)
    );
    terminal.market_universe = config::MarketUniverseConfig::hip3_dex("xyz");
    terminal.refresh_live_watchlist_row_caches();
    assert!(
        terminal
            .live_watchlists
            .values()
            .all(|watchlist| watchlist.row_cache.is_empty())
    );

    terminal.live_watchlists.clear();
    terminal.refresh_live_watchlist_row_caches();
    terminal.refresh_live_watchlist_row_cache(1);
    assert!(terminal.live_watchlists.is_empty());
}
