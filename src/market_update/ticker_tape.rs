use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;
use std::collections::{BTreeSet, HashMap, HashSet};

// ---------------------------------------------------------------------------
// Ticker Tape Context Refresh
// ---------------------------------------------------------------------------

const TICKER_TAPE_CONTEXT_REFRESH_MS: u64 = 300_000;

impl TradingTerminal {
    pub(crate) fn request_ticker_tape_context_refresh(&mut self, force: bool) -> Task<Message> {
        if !self.ticker_tape_enabled {
            self.invalidate_ticker_tape_context_request();
            return Task::none();
        }

        let symbols = self.ticker_tape_context_symbols();
        if symbols.is_empty() {
            self.ticker_tape_ctxs.clear();
            self.invalidate_ticker_tape_context_request();
            self.ticker_tape_contexts_last_fetch_ms = None;
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let contexts_stale = self
            .ticker_tape_contexts_last_fetch_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= TICKER_TAPE_CONTEXT_REFRESH_MS);
        let contexts_missing = symbols
            .iter()
            .any(|symbol| !self.ticker_tape_ctxs.contains_key(symbol));

        if self.ticker_tape_contexts_loading {
            if force || symbols != self.ticker_tape_contexts_request_symbols {
                self.ticker_tape_contexts_refresh_pending = true;
            }
            return Task::none();
        }

        if !force && !contexts_stale && !contexts_missing {
            return Task::none();
        }

        self.ticker_tape_contexts_request_id =
            self.ticker_tape_contexts_request_id.saturating_add(1);
        let request_id = self.ticker_tape_contexts_request_id;
        let requested_symbols = symbols.clone();
        self.ticker_tape_contexts_request_symbols = requested_symbols.clone();
        self.ticker_tape_contexts_refresh_pending = false;
        self.ticker_tape_contexts_loading = true;
        Task::perform(api::fetch_watchlist_contexts(symbols), move |result| {
            Message::TickerTapeContextsLoaded(request_id, requested_symbols.clone(), now_ms, result)
        })
    }

    pub(super) fn update_ticker_tape_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TickerTapeRefreshTick => self.request_ticker_tape_context_refresh(false),
            Message::TickerTapeContextsLoaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ) => self.apply_ticker_tape_contexts_loaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ),
            _ => Task::none(),
        }
    }

    fn ticker_tape_context_symbols(&self) -> Vec<String> {
        self.favourite_symbols
            .iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn apply_ticker_tape_contexts_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<api::WatchlistContextsResponse, String>,
    ) -> Task<Message> {
        if !self.ticker_tape_contexts_loading
            || request_id != self.ticker_tape_contexts_request_id
            || requested_symbols != self.ticker_tape_contexts_request_symbols
        {
            return Task::none();
        }

        let current_symbols: HashSet<_> = self.ticker_tape_context_symbols().into_iter().collect();
        let requested_symbol_set: HashSet<_> = requested_symbols.into_iter().collect();
        let has_current_requested_symbol = current_symbols
            .iter()
            .any(|symbol| requested_symbol_set.contains(symbol));
        let preserved_contexts: HashMap<String, api::WatchlistContext> = self
            .ticker_tape_ctxs
            .iter()
            .filter(|(symbol, _)| {
                current_symbols.contains(*symbol) && !requested_symbol_set.contains(*symbol)
            })
            .map(|(symbol, context)| (symbol.clone(), context.clone()))
            .collect();
        let scoped_contexts = match result {
            Ok(response) => {
                let mut scoped_contexts = preserved_contexts;
                if !response.partial_errors.is_empty() {
                    scoped_contexts.extend(
                        self.ticker_tape_ctxs
                            .iter()
                            .filter(|(symbol, _)| {
                                requested_symbol_set.contains(*symbol)
                                    && current_symbols.contains(*symbol)
                            })
                            .map(|(symbol, context)| (symbol.clone(), context.clone())),
                    );
                }
                scoped_contexts.extend(response.contexts.into_iter().filter(|(symbol, _)| {
                    requested_symbol_set.contains(symbol) && current_symbols.contains(symbol)
                }));
                Some(scoped_contexts)
            }
            Err(_) if has_current_requested_symbol => {
                self.ticker_tape_ctxs
                    .retain(|symbol, _| current_symbols.contains(symbol));
                None
            }
            Err(_) => Some(preserved_contexts),
        };
        let refresh_pending = self.ticker_tape_contexts_refresh_pending;
        self.ticker_tape_contexts_refresh_pending = false;
        self.ticker_tape_contexts_request_symbols.clear();
        self.ticker_tape_contexts_loading = false;

        if let Some(contexts) = scoped_contexts {
            self.ticker_tape_contexts_last_fetch_ms = Some(requested_at);
            self.ticker_tape_ctxs = contexts;
        }

        if refresh_pending {
            self.request_ticker_tape_context_refresh(true)
        } else {
            Task::none()
        }
    }

    fn invalidate_ticker_tape_context_request(&mut self) {
        self.ticker_tape_contexts_request_id =
            self.ticker_tape_contexts_request_id.saturating_add(1);
        self.ticker_tape_contexts_request_symbols.clear();
        self.ticker_tape_contexts_refresh_pending = false;
        self.ticker_tape_contexts_loading = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::WatchlistContext;

    fn context(day_vlm: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: Some(day_vlm),
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
        terminal
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
}
