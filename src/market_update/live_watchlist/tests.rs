use super::*;
use crate::api::WatchlistContext;
use crate::config;
use crate::market_state::LiveWatchlistInstance;
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;

fn context(day_vlm: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        mark_px: None,
        day_vlm: Some(day_vlm),
        open_interest_notional: None,
    }
}

fn terminal_with_live_watchlist(symbols: &[&str]) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    let id = 1;
    let (panes, _) = pane_grid::State::new(PaneKind::LiveWatchlist(id));
    terminal.panes = panes;
    terminal.live_watchlists.clear();
    terminal.live_watchlists.insert(
        id,
        LiveWatchlistInstance {
            id,
            preset_id: None,
            symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
            search_query: String::new(),
            sort_column: Default::default(),
            sort_direction: Default::default(),
            visible_columns: config::default_live_watchlist_columns(),
            row_cache: Vec::new(),
        },
    );
    terminal.live_watchlist_ctxs.clear();
    terminal.live_watchlist_history.clear();
    terminal.live_watchlist_contexts_loading = false;
    terminal.live_watchlist_history_loading = false;
    terminal.live_watchlist_contexts_request_id = 0;
    terminal.live_watchlist_contexts_request_symbols.clear();
    terminal.live_watchlist_contexts_refresh_pending = false;
    terminal.live_watchlist_history_request_id = 0;
    terminal.live_watchlist_history_request_symbols.clear();
    terminal.live_watchlist_history_refresh_pending = false;
    terminal.live_watchlist_contexts_last_fetch_ms = None;
    terminal.live_watchlist_history_loaded_at.clear();
    terminal.live_watchlist_status = None;
    terminal
}

#[test]
fn live_watchlist_scope_change_queues_current_context_scope_after_in_flight_result() {
    let mut terminal = terminal_with_live_watchlist(&["BTC"]);

    let _task = terminal.request_live_watchlist_refresh(true);
    let stale_request_id = terminal.live_watchlist_contexts_request_id;
    assert!(terminal.live_watchlist_contexts_loading);
    assert_eq!(
        terminal.live_watchlist_contexts_request_symbols,
        vec!["BTC".to_string()]
    );

    let _task =
        terminal.update_live_watchlist_market(Message::LiveWatchlistAddSymbol(1, "ETH".into()));
    assert!(terminal.live_watchlist_contexts_refresh_pending);

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
        stale_request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(
        terminal.live_watchlist_contexts_loading,
        "queued refresh should start for the current live-watchlist scope"
    );
    assert!(!terminal.live_watchlist_contexts_refresh_pending);
    assert_eq!(
        terminal.live_watchlist_contexts_request_symbols,
        vec!["BTC".to_string(), "ETH".to_string()]
    );
}

#[test]
fn live_watchlist_history_result_does_not_mark_removed_symbols_loaded() {
    let mut terminal = terminal_with_live_watchlist(&["BTC"]);

    let _task = terminal.request_live_watchlist_refresh(true);
    let request_id = terminal.live_watchlist_history_request_id;
    assert!(terminal.live_watchlist_history_loading);

    terminal
        .live_watchlists
        .get_mut(&1)
        .expect("watchlist should exist")
        .symbols
        .clear();

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistHistoryLoaded(
        request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), (1.0, 2.0, 3.0))])),
    ));

    assert!(!terminal.live_watchlist_history_loading);
    assert!(!terminal.live_watchlist_history.contains_key("BTC"));
    assert!(
        !terminal
            .live_watchlist_history_loaded_at
            .contains_key("BTC")
    );
}

#[test]
fn live_watchlist_context_result_filters_to_requested_current_symbols() {
    let mut terminal = terminal_with_live_watchlist(&["BTC", "ETH"]);
    terminal
        .live_watchlist_ctxs
        .insert("ETH".to_string(), context(2.0));
    terminal.live_watchlist_contexts_loading = true;
    terminal.live_watchlist_contexts_request_id = 7;
    terminal.live_watchlist_contexts_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
        7,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([
            ("BTC".to_string(), context(1.0)),
            ("DOGE".to_string(), context(3.0)),
        ])
        .into()),
    ));

    assert_eq!(terminal.live_watchlist_ctxs.len(), 2);
    assert_eq!(
        terminal
            .live_watchlist_ctxs
            .get("BTC")
            .and_then(|ctx| ctx.day_vlm),
        Some(1.0)
    );
    assert_eq!(
        terminal
            .live_watchlist_ctxs
            .get("ETH")
            .and_then(|ctx| ctx.day_vlm),
        Some(2.0)
    );
    assert!(!terminal.live_watchlist_ctxs.contains_key("DOGE"));
}

#[test]
fn live_watchlist_partial_context_keeps_omitted_requested_last_known_value() {
    let mut terminal = terminal_with_live_watchlist(&["BTC", "@107"]);
    terminal
        .live_watchlist_ctxs
        .insert("@107".to_string(), context(9.0));
    terminal.live_watchlist_contexts_loading = true;
    terminal.live_watchlist_contexts_request_id = 7;
    terminal.live_watchlist_contexts_request_symbols = vec!["BTC".to_string(), "@107".to_string()];

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
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
            .live_watchlist_ctxs
            .get("@107")
            .and_then(|ctx| ctx.day_vlm),
        Some(9.0)
    );
}

#[test]
fn live_watchlist_history_result_ignores_unrequested_payload_symbols() {
    let mut terminal = terminal_with_live_watchlist(&["BTC"]);
    terminal.live_watchlist_history_loading = true;
    terminal.live_watchlist_history_request_id = 7;
    terminal.live_watchlist_history_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistHistoryLoaded(
        7,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([
            ("BTC".to_string(), (1.0, 2.0, 3.0)),
            ("ETH".to_string(), (4.0, 5.0, 6.0)),
        ])),
    ));

    assert_eq!(
        terminal.live_watchlist_history.get("BTC"),
        Some(&(1.0, 2.0, 3.0))
    );
    assert!(!terminal.live_watchlist_history.contains_key("ETH"));
    assert_eq!(
        terminal.live_watchlist_history_loaded_at.get("BTC"),
        Some(&10)
    );
    assert!(
        !terminal
            .live_watchlist_history_loaded_at
            .contains_key("ETH")
    );
}

#[test]
fn live_watchlist_history_result_removes_stale_requested_symbol_when_payload_omits_it() {
    let mut terminal = terminal_with_live_watchlist(&["BTC", "ETH"]);
    terminal
        .live_watchlist_history
        .insert("BTC".to_string(), (1.0, 2.0, 3.0));
    terminal
        .live_watchlist_history
        .insert("ETH".to_string(), (4.0, 5.0, 6.0));
    terminal
        .live_watchlist_history_loaded_at
        .insert("BTC".to_string(), 5);
    terminal
        .live_watchlist_history_loaded_at
        .insert("ETH".to_string(), 5);
    terminal.live_watchlist_history_loading = true;
    terminal.live_watchlist_history_request_id = 7;
    terminal.live_watchlist_history_request_symbols = vec!["BTC".to_string(), "ETH".to_string()];

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistHistoryLoaded(
        7,
        vec!["BTC".to_string(), "ETH".to_string()],
        10,
        Ok(HashMap::from([("ETH".to_string(), (7.0, 8.0, 9.0))])),
    ));

    assert!(!terminal.live_watchlist_history_loading);
    assert!(!terminal.live_watchlist_history.contains_key("BTC"));
    assert_eq!(
        terminal.live_watchlist_history.get("ETH"),
        Some(&(7.0, 8.0, 9.0))
    );
    assert_eq!(
        terminal.live_watchlist_history_loaded_at.get("BTC"),
        Some(&10)
    );
    assert_eq!(
        terminal.live_watchlist_history_loaded_at.get("ETH"),
        Some(&10)
    );
}

#[test]
fn duplicate_live_watchlist_context_result_is_ignored_after_completion() {
    let mut terminal = terminal_with_live_watchlist(&["BTC"]);

    let _task = terminal.request_live_watchlist_refresh(true);
    let request_id = terminal.live_watchlist_contexts_request_id;
    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));
    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        11,
        Ok(HashMap::from([("BTC".to_string(), context(2.0))]).into()),
    ));

    assert!(!terminal.live_watchlist_contexts_loading);
    assert_eq!(
        terminal
            .live_watchlist_ctxs
            .get("BTC")
            .and_then(|ctx| ctx.day_vlm),
        Some(1.0)
    );
    assert_eq!(terminal.live_watchlist_contexts_last_fetch_ms, Some(10));
}

#[test]
fn stale_live_watchlist_context_result_does_not_clear_current_request() {
    let mut terminal = terminal_with_live_watchlist(&["ETH"]);
    terminal.live_watchlist_contexts_loading = true;
    terminal.live_watchlist_contexts_request_id = 2;
    terminal.live_watchlist_contexts_request_symbols = vec!["ETH".to_string()];

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
        1,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(terminal.live_watchlist_contexts_loading);
    assert_eq!(terminal.live_watchlist_contexts_request_id, 2);
    assert_eq!(
        terminal.live_watchlist_contexts_request_symbols,
        vec!["ETH".to_string()]
    );
    assert!(terminal.live_watchlist_ctxs.is_empty());
    assert_eq!(terminal.live_watchlist_contexts_last_fetch_ms, None);
}

#[test]
fn live_watchlist_context_error_does_not_advance_last_fetch_time() {
    let mut terminal = terminal_with_live_watchlist(&["BTC"]);
    terminal.live_watchlist_contexts_last_fetch_ms = Some(10);
    terminal.live_watchlist_contexts_loading = true;
    terminal.live_watchlist_contexts_request_id = 7;
    terminal.live_watchlist_contexts_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
        7,
        vec!["BTC".to_string()],
        20,
        Err("network".to_string()),
    ));

    assert!(!terminal.live_watchlist_contexts_loading);
    assert_eq!(terminal.live_watchlist_contexts_last_fetch_ms, Some(10));
    assert_eq!(
        terminal
            .live_watchlist_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("Watchlist context refresh failed: network", true))
    );
}
