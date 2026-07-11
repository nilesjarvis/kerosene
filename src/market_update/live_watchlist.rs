mod controls;
mod results;
mod symbols;

use crate::api::WatchlistContext;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::LiveWatchlistInstance;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use std::collections::{HashMap, HashSet};

use self::controls::{apply_column_toggle, apply_sort_change};
use self::results::{apply_contexts_loaded, apply_history_loaded};
use self::symbols::{add_watchlist_symbol, remove_watchlist_symbol, update_watchlist_search};

impl TradingTerminal {
    pub(crate) fn update_live_watchlist_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LiveWatchlistSortChanged(id, col) => {
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    apply_sort_change(watchlist, col);
                }
                self.refresh_live_watchlist_row_cache(id);
                self.persist_config();
                Task::none()
            }
            Message::LiveWatchlistColumnToggled(id, column, enabled) => {
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    apply_column_toggle(watchlist, column, enabled);
                }
                self.refresh_live_watchlist_row_cache(id);
                self.persist_config();
                Task::none()
            }
            Message::ToggleLiveWatchlistSettings(id) => {
                let opening = self.live_watchlist_settings_menu_open != Some(id);
                if opening {
                    self.close_chart_header_menus();
                    self.live_watchlist_settings_menu_open = Some(id);
                } else {
                    self.live_watchlist_settings_menu_open = None;
                }
                Task::none()
            }
            Message::AddLiveWatchlistPane => {
                self.add_widget_menu_open = false;
                let Some(focus) = self.add_target_pane() else {
                    self.push_toast(
                        "Could not add Live Watchlist: no pane is available".to_string(),
                        true,
                    );
                    return Task::none();
                };

                let id = crate::ws::now_ms();
                self.live_watchlists.insert(
                    id,
                    LiveWatchlistInstance {
                        id,
                        symbols: Vec::new(),
                        search_query: String::new(),
                        sort_column: Default::default(),
                        sort_direction: Default::default(),
                        visible_columns: config::default_live_watchlist_columns(),
                        row_cache: Vec::new(),
                    },
                );
                if self
                    .add_pane_to_target(
                        self.add_widget_axis(),
                        focus,
                        PaneKind::LiveWatchlist(id),
                        "Live Watchlist",
                    )
                    .is_none()
                {
                    self.live_watchlists.remove(&id);
                }
                Task::none()
            }
            Message::LiveWatchlistSearchChanged(id, query) => {
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    update_watchlist_search(watchlist, query);
                }
                Task::none()
            }
            Message::LiveWatchlistAddSymbol(id, symbol) => {
                if self.symbol_key_is_hidden(&symbol) {
                    self.live_watchlist_status =
                        Some((format!("{symbol} is hidden in Settings > Risk"), true));
                    return Task::none();
                }
                if self
                    .exchange_symbols
                    .iter()
                    .find(|exchange_symbol| exchange_symbol.key == symbol)
                    .is_some_and(|exchange_symbol| !exchange_symbol.is_user_selectable_market())
                {
                    self.live_watchlist_status =
                        Some((format!("{symbol} is not a tradable market"), true));
                    return Task::none();
                }
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    add_watchlist_symbol(watchlist, symbol);
                }
                self.refresh_live_watchlist_row_cache(id);
                self.persist_config();
                self.request_live_watchlist_refresh(true)
            }
            Message::LiveWatchlistRemoveSymbol(id, symbol) => {
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    remove_watchlist_symbol(watchlist, &symbol);
                }
                self.refresh_live_watchlist_row_cache(id);
                self.persist_config();
                Task::none()
            }
            Message::LiveWatchlistRefreshTick => self.request_live_watchlist_refresh(false),
            Message::LiveWatchlistContextsLoaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ) => self.apply_live_watchlist_contexts_loaded(
                request_id,
                requested_symbols,
                requested_at,
                result.into_result(),
            ),
            Message::LiveWatchlistHistoryLoaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ) => self.apply_live_watchlist_history_loaded(
                request_id,
                requested_symbols,
                requested_at,
                result.into_result(),
            ),
            _ => Task::none(),
        }
    }

    fn apply_live_watchlist_contexts_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<crate::api::WatchlistContextsResponse, String>,
    ) -> Task<Message> {
        if !self.live_watchlist_contexts_loading
            || request_id != self.live_watchlist_contexts_request_id
            || requested_symbols != self.live_watchlist_contexts_request_symbols
        {
            return Task::none();
        }

        let current_symbols = self.current_live_watchlist_symbol_set();
        let requested_symbol_set: HashSet<_> = requested_symbols.iter().cloned().collect();
        let has_current_requested_symbol = current_symbols
            .iter()
            .any(|symbol| requested_symbol_set.contains(symbol));
        let preserved_contexts: HashMap<String, WatchlistContext> = self
            .live_watchlist_ctxs
            .iter()
            .filter(|(symbol, _)| {
                current_symbols.contains(*symbol) && !requested_symbol_set.contains(*symbol)
            })
            .map(|(symbol, context)| (symbol.clone(), context.clone()))
            .collect();
        let scoped_result = match result {
            Ok(mut response) => {
                let mut scoped_contexts = preserved_contexts;
                if !response.partial_errors.is_empty() {
                    scoped_contexts.extend(
                        self.live_watchlist_ctxs
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
                response.contexts = scoped_contexts;
                Ok(response)
            }
            Err(error) if has_current_requested_symbol => {
                self.live_watchlist_ctxs
                    .retain(|symbol, _| current_symbols.contains(symbol));
                Err(error)
            }
            Err(_) => Ok(preserved_contexts.into()),
        };
        let refresh_pending = self.live_watchlist_contexts_refresh_pending;
        self.live_watchlist_contexts_refresh_pending = false;
        self.live_watchlist_contexts_request_symbols.clear();

        apply_contexts_loaded(
            &mut self.live_watchlist_contexts_loading,
            &mut self.live_watchlist_contexts_last_fetch_ms,
            &mut self.live_watchlist_ctxs,
            &mut self.live_watchlist_status,
            requested_at,
            scoped_result,
        );
        self.refresh_live_watchlist_row_caches();

        if refresh_pending {
            self.request_live_watchlist_refresh(false)
        } else {
            Task::none()
        }
    }

    fn apply_live_watchlist_history_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<HashMap<String, (f64, f64, f64)>, String>,
    ) -> Task<Message> {
        if !self.live_watchlist_history_loading
            || request_id != self.live_watchlist_history_request_id
            || requested_symbols != self.live_watchlist_history_request_symbols
        {
            return Task::none();
        }

        let current_symbols = self.current_live_watchlist_symbol_set();
        self.live_watchlist_history
            .retain(|symbol, _| current_symbols.contains(symbol));
        self.live_watchlist_history_loaded_at
            .retain(|symbol, _| current_symbols.contains(symbol));
        let scoped_requested_symbols: Vec<_> = requested_symbols
            .into_iter()
            .filter(|symbol| {
                current_symbols.contains(symbol)
                    && self
                        .live_watchlist_history_loaded_at
                        .get(symbol)
                        .is_none_or(|last| requested_at >= *last)
            })
            .collect();
        let scoped_requested_symbol_set: HashSet<_> =
            scoped_requested_symbols.iter().cloned().collect();
        let scoped_result = if scoped_requested_symbol_set.is_empty() {
            Ok(HashMap::new())
        } else {
            result.map(|history| {
                history
                    .into_iter()
                    .filter(|(symbol, _)| scoped_requested_symbol_set.contains(symbol))
                    .collect()
            })
        };
        let refresh_pending = self.live_watchlist_history_refresh_pending;
        self.live_watchlist_history_refresh_pending = false;
        self.live_watchlist_history_request_symbols.clear();

        apply_history_loaded(
            &mut self.live_watchlist_history_loading,
            &mut self.live_watchlist_history_loaded_at,
            &mut self.live_watchlist_history,
            &mut self.live_watchlist_status,
            scoped_requested_symbols,
            requested_at,
            scoped_result,
        );
        self.refresh_live_watchlist_row_caches();

        if refresh_pending {
            self.request_live_watchlist_refresh(false)
        } else {
            Task::none()
        }
    }

    fn current_live_watchlist_symbol_set(&self) -> HashSet<String> {
        self.watched_live_watchlist_symbols().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::WatchlistContext;
    use iced::widget::pane_grid;

    fn context(day_vlm: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: Some(day_vlm),
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
            Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()).into(),
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
            Ok(HashMap::from([("BTC".to_string(), (1.0, 2.0, 3.0))])).into(),
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
            .into())
            .into(),
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
        terminal.live_watchlist_contexts_request_symbols =
            vec!["BTC".to_string(), "@107".to_string()];

        let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
            7,
            vec!["BTC".to_string(), "@107".to_string()],
            10,
            Ok(crate::api::WatchlistContextsResponse {
                contexts: HashMap::from([("BTC".to_string(), context(1.0))]),
                partial_errors: vec!["spot: HTTP 503".to_string()],
            })
            .into(),
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
            ]))
            .into(),
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
        terminal.live_watchlist_history_request_symbols =
            vec!["BTC".to_string(), "ETH".to_string()];

        let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistHistoryLoaded(
            7,
            vec!["BTC".to_string(), "ETH".to_string()],
            10,
            Ok(HashMap::from([("ETH".to_string(), (7.0, 8.0, 9.0))])).into(),
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
            Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()).into(),
        ));
        let _task = terminal.update_live_watchlist_market(Message::LiveWatchlistContextsLoaded(
            request_id,
            vec!["BTC".to_string()],
            11,
            Ok(HashMap::from([("BTC".to_string(), context(2.0))]).into()).into(),
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
            Ok(HashMap::from([("BTC".to_string(), context(1.0))]).into()).into(),
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
            Err("network".to_string()).into(),
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
}
