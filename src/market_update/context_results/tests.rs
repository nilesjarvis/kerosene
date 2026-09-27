use crate::api::{WatchlistContext, WatchlistContextsResponse};
use crate::app_state::TradingTerminal;
use crate::market_state::LiveWatchlistInstance;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy)]
enum Surface {
    Watchlist,
    Tape,
}

#[derive(Clone, Copy)]
enum Response {
    Complete,
    Partial,
    Error,
}

fn context(value: f64) -> WatchlistContext {
    WatchlistContext {
        funding: Some(value / 100.0),
        prev_day_px: Some(value),
        mark_px: Some(value + 1.0),
        day_vlm: Some(value * 10.0),
        open_interest_notional: Some(value * 100.0),
    }
}

fn initial_contexts() -> HashMap<String, WatchlistContext> {
    [
        ("KEEP", 10.0),
        ("REFRESH", 20.0),
        ("MISSING", 30.0),
        ("REMOVED", 40.0),
    ]
    .into_iter()
    .map(|(key, value)| (key.into(), context(value)))
    .collect()
}

fn terminal(surface: Surface, current: &[&str], requested: &[&str]) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.market_universe = crate::config::MarketUniverseConfig::All;
    terminal.muted_tickers.clear();
    let symbols: Vec<_> = current.iter().map(|key| (*key).to_string()).collect();
    let requested: Vec<_> = requested.iter().map(|key| (*key).to_string()).collect();
    terminal.live_watchlist_status =
        Some(("Watchlist context refresh failed: previous".into(), true));
    match surface {
        Surface::Watchlist => {
            terminal.panes = pane_grid::State::new(PaneKind::LiveWatchlist(1)).0;
            terminal.live_watchlists = HashMap::from([(
                1,
                LiveWatchlistInstance {
                    id: 1,
                    preset_id: None,
                    symbols,
                    search_query: String::new(),
                    sort_column: Default::default(),
                    sort_direction: Default::default(),
                    visible_columns: crate::config::default_live_watchlist_columns(),
                    row_cache: Vec::new(),
                },
            )]);
            terminal.live_watchlist_ctxs = initial_contexts();
            terminal.live_watchlist_contexts_loading = true;
            terminal.live_watchlist_contexts_request_id = 7;
            terminal.live_watchlist_contexts_request_symbols = requested;
            terminal.live_watchlist_contexts_refresh_pending = false;
            terminal.live_watchlist_contexts_last_fetch_ms = Some(5);
        }
        Surface::Tape => {
            terminal.ticker_tape_enabled = true;
            terminal.favourite_symbols = symbols;
            terminal.ticker_tape_ctxs = initial_contexts();
            terminal.ticker_tape_contexts_loading = true;
            terminal.ticker_tape_contexts_request_id = 7;
            terminal.ticker_tape_contexts_request_symbols = requested;
            terminal.ticker_tape_contexts_refresh_pending = false;
            terminal.ticker_tape_contexts_last_fetch_ms = Some(5);
        }
    }
    terminal
}

fn message(surface: Surface, id: u64, requested: &[&str], response: Response) -> Message {
    let result = match response {
        Response::Error => Err("network".into()),
        Response::Complete | Response::Partial => Ok(WatchlistContextsResponse {
            contexts: [
                ("REFRESH", 99.0),
                ("REMOVED", 88.0),
                ("KEEP", 77.0),
                ("NEW", 66.0),
                ("OUTSIDE", 55.0),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), context(value)))
            .collect(),
            partial_errors: if matches!(response, Response::Partial) {
                vec!["family unavailable".into()]
            } else {
                Vec::new()
            },
        }),
    };
    let requested = requested.iter().map(|key| (*key).to_string()).collect();
    match surface {
        Surface::Watchlist => Message::LiveWatchlistContextsLoaded(id, requested, 42, result),
        Surface::Tape => Message::TickerTapeContextsLoaded(id, requested, 42, result),
    }
}

fn snapshot(terminal: &TradingTerminal, surface: Surface) -> serde_json::Value {
    let (contexts, loading, id, requested, pending, last_fetch) = match surface {
        Surface::Watchlist => (
            &terminal.live_watchlist_ctxs,
            terminal.live_watchlist_contexts_loading,
            terminal.live_watchlist_contexts_request_id,
            &terminal.live_watchlist_contexts_request_symbols,
            terminal.live_watchlist_contexts_refresh_pending,
            terminal.live_watchlist_contexts_last_fetch_ms,
        ),
        Surface::Tape => (
            &terminal.ticker_tape_ctxs,
            terminal.ticker_tape_contexts_loading,
            terminal.ticker_tape_contexts_request_id,
            &terminal.ticker_tape_contexts_request_symbols,
            terminal.ticker_tape_contexts_refresh_pending,
            terminal.ticker_tape_contexts_last_fetch_ms,
        ),
    };
    serde_json::json!({
        "contexts": contexts, "loading": loading, "id": id, "requested": requested,
        "pending": pending, "last_fetch": last_fetch,
        "watchlist_status": terminal.live_watchlist_status,
    })
}

#[test]
fn both_context_consumers_preserve_scope_and_completion_policy() {
    struct Case<'a> {
        response: Response,
        current: &'a [&'a str],
        requested: &'a [&'a str],
        expected: &'a [(&'a str, f64)],
        last_fetch: u64,
        status: Option<&'a str>,
    }
    let current = &["KEEP", "REFRESH", "MISSING", "NEW"];
    let requested = &["REFRESH", "MISSING", "REMOVED"];
    let retained = &[("KEEP", 10.0), ("REFRESH", 20.0), ("MISSING", 30.0)];
    let cases = [
        Case {
            response: Response::Complete,
            current,
            requested,
            expected: &[("KEEP", 10.0), ("REFRESH", 99.0)],
            last_fetch: 42,
            status: None,
        },
        Case {
            response: Response::Partial,
            current,
            requested,
            expected: &[("KEEP", 10.0), ("REFRESH", 99.0), ("MISSING", 30.0)],
            last_fetch: 42,
            status: Some("Watchlist context refresh partially failed: family unavailable"),
        },
        Case {
            response: Response::Error,
            current,
            requested,
            expected: retained,
            last_fetch: 5,
            status: Some("Watchlist context refresh failed: network"),
        },
        Case {
            response: Response::Error,
            current,
            requested: &["REMOVED"],
            expected: retained,
            last_fetch: 42,
            status: None,
        },
        Case {
            response: Response::Complete,
            current: &[],
            requested,
            expected: &[],
            last_fetch: 42,
            status: None,
        },
        Case {
            response: Response::Partial,
            current: &[],
            requested,
            expected: &[],
            last_fetch: 42,
            status: Some("Watchlist context refresh partially failed: family unavailable"),
        },
        Case {
            response: Response::Error,
            current: &[],
            requested,
            expected: &[],
            last_fetch: 42,
            status: None,
        },
        Case {
            response: Response::Error,
            current: &["NEW"],
            requested: &["NEW"],
            expected: &[],
            last_fetch: 5,
            status: Some("Watchlist context refresh failed: network"),
        },
        Case {
            response: Response::Complete,
            current,
            requested: &[],
            expected: retained,
            last_fetch: 42,
            status: None,
        },
        Case {
            response: Response::Error,
            current,
            requested: &[],
            expected: retained,
            last_fetch: 42,
            status: None,
        },
        Case {
            response: Response::Complete,
            current,
            requested: &["REFRESH", "MISSING", "REMOVED", "REFRESH"],
            expected: &[("KEEP", 10.0), ("REFRESH", 99.0)],
            last_fetch: 42,
            status: None,
        },
    ];
    for surface in [Surface::Watchlist, Surface::Tape] {
        for (index, case) in cases.iter().enumerate() {
            let mut terminal = terminal(surface, case.current, case.requested);
            let _task = terminal.update_market(message(surface, 7, case.requested, case.response));
            let actual = snapshot(&terminal, surface);
            let expected: BTreeMap<_, _> = case
                .expected
                .iter()
                .map(|(key, value)| (*key, context(*value)))
                .collect();
            assert_eq!(
                actual["contexts"],
                serde_json::to_value(expected).expect("contexts"),
                "{surface:?}, case {index}"
            );
            assert_eq!(
                actual["last_fetch"], case.last_fetch,
                "{surface:?}, case {index}"
            );
            assert_eq!(actual["loading"], false);
            assert_eq!(actual["pending"], false);
            assert_eq!(actual["id"], 7);
            assert_eq!(actual["requested"], serde_json::json!([]));
            let expected_status = match surface {
                Surface::Watchlist => case.status,
                Surface::Tape => Some("Watchlist context refresh failed: previous"),
            }
            .map(|status| (status, true));
            assert_eq!(
                actual["watchlist_status"],
                serde_json::json!(expected_status),
                "{surface:?}, case {index}"
            );
        }
    }
}

#[test]
fn rejected_results_leave_both_context_consumers_untouched() {
    for surface in [Surface::Watchlist, Surface::Tape] {
        for rejection in 0..3 {
            let mut terminal = terminal(surface, &["REFRESH"], &["REFRESH"]);
            if rejection == 0 {
                terminal.live_watchlist_contexts_loading = false;
                terminal.ticker_tape_contexts_loading = false;
            }
            let before = snapshot(&terminal, surface);
            let id = if rejection == 1 { 6 } else { 7 };
            let requested = if rejection == 2 {
                &["OTHER"]
            } else {
                &["REFRESH"]
            };
            let _task = terminal.update_market(message(surface, id, requested, Response::Complete));
            assert_eq!(
                snapshot(&terminal, surface),
                before,
                "{surface:?}, rejection {rejection}"
            );
        }
    }
}
