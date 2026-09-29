use super::*;
use crate::api::WatchlistContext;
use std::collections::HashMap;

fn context(day_vlm: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        mark_px: None,
        day_vlm: Some(day_vlm),
        open_interest_notional: None,
    }
}

fn terminal_with_ticker_tape(symbols: &[&str]) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.ticker_tape_enabled = true;
    terminal.favourite_symbols = symbols.iter().map(|symbol| (*symbol).to_string()).collect();
    terminal.ticker_tape_ctxs.clear();
    terminal.ticker_tape_contexts_loading = false;
    terminal.ticker_tape_contexts_request_id = 0;
    terminal.ticker_tape_contexts_request_symbols.clear();
    terminal.ticker_tape_contexts_refresh_pending = false;
    terminal.ticker_tape_contexts_last_fetch_ms = None;
    terminal.ticker_tape_exchange_stats = None;
    terminal.ticker_tape_exchange_stats_loading = false;
    terminal.ticker_tape_exchange_stats_request_id = 0;
    terminal.ticker_tape_exchange_stats_last_fetch_ms = None;
    terminal
}

fn exchange_stats(
    volume_24h_notional_usd: f64,
    open_interest_notional_usd: f64,
) -> api::ExchangeStats {
    api::ExchangeStats {
        volume_24h_notional_usd,
        open_interest_notional_usd,
    }
}

#[test]
fn ticker_tape_refresh_requests_exchange_stats_without_favourites() {
    let mut terminal = terminal_with_ticker_tape(&[]);

    let _task = terminal.request_ticker_tape_refresh(true);

    assert!(terminal.ticker_tape_exchange_stats_loading);
    assert_eq!(terminal.ticker_tape_exchange_stats_request_id, 1);
    assert!(!terminal.ticker_tape_contexts_loading);
}

#[test]
fn ticker_tape_exchange_stats_result_updates_complete_snapshot() {
    let mut terminal = terminal_with_ticker_tape(&[]);
    let _task = terminal.request_ticker_tape_exchange_stats_refresh(true);
    let request_id = terminal.ticker_tape_exchange_stats_request_id;

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeExchangeStatsLoaded(
        request_id,
        10,
        Ok(exchange_stats(4_250_000_000.0, 11_750_000_000.0)),
    ));

    assert!(!terminal.ticker_tape_exchange_stats_loading);
    assert_eq!(
        terminal.ticker_tape_exchange_stats,
        Some(exchange_stats(4_250_000_000.0, 11_750_000_000.0))
    );
    assert_eq!(terminal.ticker_tape_exchange_stats_last_fetch_ms, Some(10));
}

#[test]
fn ticker_tape_exchange_stats_error_preserves_last_complete_snapshot() {
    let mut terminal = terminal_with_ticker_tape(&[]);
    terminal.ticker_tape_exchange_stats = Some(exchange_stats(4_000_000_000.0, 11_000_000_000.0));
    terminal.ticker_tape_exchange_stats_last_fetch_ms = Some(5);
    let _task = terminal.request_ticker_tape_exchange_stats_refresh(true);
    let request_id = terminal.ticker_tape_exchange_stats_request_id;

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeExchangeStatsLoaded(
        request_id,
        10,
        Err("spot: HTTP 503".to_string()),
    ));

    assert!(!terminal.ticker_tape_exchange_stats_loading);
    assert_eq!(
        terminal.ticker_tape_exchange_stats,
        Some(exchange_stats(4_000_000_000.0, 11_000_000_000.0))
    );
    assert_eq!(terminal.ticker_tape_exchange_stats_last_fetch_ms, Some(5));
}

#[test]
fn ticker_tape_scope_change_queues_current_scope_after_in_flight_result() {
    let mut terminal = terminal_with_ticker_tape(&["BTC"]);

    let _task = terminal.request_ticker_tape_context_refresh(true);
    let stale_request_id = terminal.ticker_tape_contexts_request_id;
    assert!(terminal.ticker_tape_contexts_loading);
    assert_eq!(
        terminal.ticker_tape_contexts_request_symbols,
        vec!["BTC".to_string()]
    );

    terminal.favourite_symbols.push("ETH".to_string());
    let _task = terminal.request_ticker_tape_context_refresh(true);
    assert!(terminal.ticker_tape_contexts_refresh_pending);

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        stale_request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(
        terminal.ticker_tape_contexts_loading,
        "queued refresh should start for the current favourite-symbol scope"
    );
    assert!(!terminal.ticker_tape_contexts_refresh_pending);
    assert_eq!(
        terminal.ticker_tape_contexts_request_symbols,
        vec!["BTC".to_string(), "ETH".to_string()]
    );
}

#[test]
fn duplicate_ticker_tape_context_result_is_ignored_after_completion() {
    let mut terminal = terminal_with_ticker_tape(&["BTC"]);

    let _task = terminal.request_ticker_tape_context_refresh(true);
    let request_id = terminal.ticker_tape_contexts_request_id;
    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));
    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        11,
        Ok(HashMap::from([("BTC".to_string(), context(2.0))]).into()),
    ));

    assert!(!terminal.ticker_tape_contexts_loading);
    assert_eq!(
        terminal
            .ticker_tape_ctxs
            .get("BTC")
            .and_then(|ctx| ctx.day_vlm),
        Some(1.0)
    );
    assert_eq!(terminal.ticker_tape_contexts_last_fetch_ms, Some(10));
}

#[test]
fn ticker_tape_context_result_filters_to_requested_current_symbols() {
    let mut terminal = terminal_with_ticker_tape(&["BTC", "ETH"]);
    terminal
        .ticker_tape_ctxs
        .insert("ETH".to_string(), context(2.0));
    terminal.ticker_tape_contexts_loading = true;
    terminal.ticker_tape_contexts_request_id = 7;
    terminal.ticker_tape_contexts_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        7,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([
            ("BTC".to_string(), context(1.0)),
            ("DOGE".to_string(), context(3.0)),
        ])
        .into()),
    ));

    assert_eq!(terminal.ticker_tape_ctxs.len(), 2);
    assert_eq!(
        terminal
            .ticker_tape_ctxs
            .get("BTC")
            .and_then(|ctx| ctx.day_vlm),
        Some(1.0)
    );
    assert_eq!(
        terminal
            .ticker_tape_ctxs
            .get("ETH")
            .and_then(|ctx| ctx.day_vlm),
        Some(2.0)
    );
    assert!(!terminal.ticker_tape_ctxs.contains_key("DOGE"));
}

#[test]
fn ticker_tape_partial_context_keeps_omitted_requested_last_known_value() {
    let mut terminal = terminal_with_ticker_tape(&["BTC", "@107"]);
    terminal
        .ticker_tape_ctxs
        .insert("@107".to_string(), context(9.0));
    terminal.ticker_tape_contexts_loading = true;
    terminal.ticker_tape_contexts_request_id = 7;
    terminal.ticker_tape_contexts_request_symbols = vec!["BTC".to_string(), "@107".to_string()];

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        7,
        vec!["BTC".to_string(), "@107".to_string()],
        10,
        Ok(api::WatchlistContextsResponse {
            contexts: HashMap::from([("BTC".to_string(), context(1.0))]),
            partial_errors: vec!["spot: HTTP 503".to_string()],
        }),
    ));

    assert_eq!(
        terminal
            .ticker_tape_ctxs
            .get("@107")
            .and_then(|ctx| ctx.day_vlm),
        Some(9.0)
    );
}

#[test]
fn empty_ticker_tape_scope_invalidates_in_flight_context_result() {
    let mut terminal = terminal_with_ticker_tape(&["BTC"]);

    let _task = terminal.request_ticker_tape_context_refresh(true);
    let stale_request_id = terminal.ticker_tape_contexts_request_id;
    terminal.favourite_symbols.clear();
    let _task = terminal.request_ticker_tape_context_refresh(true);

    assert!(!terminal.ticker_tape_contexts_loading);
    assert!(terminal.ticker_tape_contexts_request_symbols.is_empty());
    assert!(terminal.ticker_tape_ctxs.is_empty());
    assert_eq!(terminal.ticker_tape_contexts_last_fetch_ms, None);

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        stale_request_id,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(terminal.ticker_tape_ctxs.is_empty());
    assert_eq!(terminal.ticker_tape_contexts_last_fetch_ms, None);
}

#[test]
fn stale_ticker_tape_context_result_does_not_clear_current_request() {
    let mut terminal = terminal_with_ticker_tape(&["ETH"]);
    terminal.ticker_tape_contexts_loading = true;
    terminal.ticker_tape_contexts_request_id = 2;
    terminal.ticker_tape_contexts_request_symbols = vec!["ETH".to_string()];

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        1,
        vec!["BTC".to_string()],
        10,
        Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()),
    ));

    assert!(terminal.ticker_tape_contexts_loading);
    assert_eq!(terminal.ticker_tape_contexts_request_id, 2);
    assert_eq!(
        terminal.ticker_tape_contexts_request_symbols,
        vec!["ETH".to_string()]
    );
    assert!(terminal.ticker_tape_ctxs.is_empty());
    assert_eq!(terminal.ticker_tape_contexts_last_fetch_ms, None);
}

#[test]
fn ticker_tape_force_refresh_while_loading_runs_followup_after_error() {
    let mut terminal = terminal_with_ticker_tape(&["BTC"]);
    terminal
        .ticker_tape_ctxs
        .insert("BTC".to_string(), context(1.0));

    let _task = terminal.request_ticker_tape_context_refresh(true);
    let request_id = terminal.ticker_tape_contexts_request_id;
    let _task = terminal.request_ticker_tape_context_refresh(true);

    assert!(terminal.ticker_tape_contexts_refresh_pending);

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        request_id,
        vec!["BTC".to_string()],
        10,
        Err("temporary failure".to_string()),
    ));

    assert!(
        terminal.ticker_tape_contexts_loading,
        "same-scope forced refresh should retry after the active request fails"
    );
    assert!(!terminal.ticker_tape_contexts_refresh_pending);
    assert_eq!(
        terminal.ticker_tape_contexts_request_symbols,
        vec!["BTC".to_string()]
    );
    assert_eq!(terminal.ticker_tape_contexts_last_fetch_ms, None);
}

#[test]
fn ticker_tape_context_error_prunes_removed_symbols_without_advancing_last_fetch() {
    let mut terminal = terminal_with_ticker_tape(&["BTC"]);
    terminal
        .ticker_tape_ctxs
        .insert("BTC".to_string(), context(1.0));
    terminal
        .ticker_tape_ctxs
        .insert("ETH".to_string(), context(2.0));
    terminal.ticker_tape_contexts_last_fetch_ms = Some(5);
    terminal.ticker_tape_contexts_loading = true;
    terminal.ticker_tape_contexts_request_id = 7;
    terminal.ticker_tape_contexts_request_symbols = vec!["BTC".to_string()];

    let _task = terminal.update_ticker_tape_market(Message::TickerTapeContextsLoaded(
        7,
        vec!["BTC".to_string()],
        10,
        Err("temporary failure".to_string()),
    ));

    assert!(!terminal.ticker_tape_contexts_loading);
    assert!(terminal.ticker_tape_ctxs.contains_key("BTC"));
    assert!(!terminal.ticker_tape_ctxs.contains_key("ETH"));
    assert_eq!(terminal.ticker_tape_contexts_last_fetch_ms, Some(5));
}
