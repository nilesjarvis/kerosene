use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::screener_state::{SCREENER_CONTEXT_REFRESH_MS, SCREENER_HISTORY_REFRESH_MS};

use iced::{Size, Task, window};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Screener Update
// ---------------------------------------------------------------------------

const SCREENER_LOADING_SYMBOLS_STATUS: &str = "Loading symbols";
const SCREENER_CONTEXT_FAILURE_PREFIX: &str = "Screener refresh failed:";
const SCREENER_HISTORY_FAILURE_PREFIX: &str = "Screener history refresh failed:";

impl TradingTerminal {
    pub(crate) fn update_screener(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenScreenerWindow => self.open_screener_window(),
            Message::RefreshScreener => self.request_screener_data_refresh(false),
            Message::ForceRefreshScreener => self.request_screener_data_refresh(true),
            Message::RefreshScreenerHistory => self.request_screener_history_refresh(),
            Message::ScreenerExchangeFilterChanged(filter) => {
                if self.screener.set_exchange_filter(filter) {
                    return self.request_screener_data_refresh(true);
                }
                Task::none()
            }
            Message::ScreenerSortChanged(column) => {
                self.screener.apply_sort_change(column);
                Task::none()
            }
            Message::ScreenerContextsLoaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ) => self.apply_screener_contexts_loaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ),
            Message::ScreenerHistoryLoaded(request_id, requested_symbols, requested_at, result) => {
                self.apply_screener_history_loaded(
                    request_id,
                    requested_symbols,
                    requested_at,
                    result,
                )
            }
            _ => Task::none(),
        }
    }

    fn open_screener_window(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        self.layout_menu_open = false;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;

        if let Some(id) = self.screener.window_id {
            return Task::batch([
                window::gain_focus(id),
                self.request_screener_data_refresh(false),
            ]);
        }

        let settings = window::Settings {
            size: Size::new(920.0, 680.0),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (id, task) = window::open(settings);
        self.screener.window_id = Some(id);

        let now_ms = Self::now_ms();
        let mids = self.all_mids.clone();
        self.record_screener_mid_samples(&mids, now_ms);

        Task::batch([
            task.map(Message::WindowOpened),
            self.request_screener_data_refresh(true),
        ])
    }

    pub(crate) fn request_screener_data_refresh(&mut self, force: bool) -> Task<Message> {
        let context_task = self.request_screener_context_refresh(force);
        let history_task = self.request_screener_history_refresh_with_force(force);
        Task::batch([context_task, history_task])
    }

    pub(crate) fn request_screener_context_refresh(&mut self, force: bool) -> Task<Message> {
        if self.screener.window_id.is_none() {
            self.screener.invalidate_context_refresh();
            return Task::none();
        }

        let symbols = self.screener_symbol_keys();
        if symbols.is_empty() {
            self.screener.status = None;
            self.screener.contexts.clear();
            self.screener.invalidate_context_refresh();
            self.screener.contexts_last_fetch_ms = None;
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let missing_contexts = symbols
            .iter()
            .any(|symbol| !self.screener.contexts.contains_key(symbol));
        let recently_refreshed = self
            .screener
            .contexts_last_fetch_ms
            .is_some_and(|last_fetch| {
                now_ms.saturating_sub(last_fetch) < SCREENER_CONTEXT_REFRESH_MS
            });
        if self.screener.contexts_loading {
            if force || symbols != self.screener.contexts_request_symbols {
                self.screener.contexts_refresh_pending = true;
            }
            return Task::none();
        }

        if recently_refreshed && !force && !missing_contexts {
            return Task::none();
        }

        self.screener.contexts_request_id = self.screener.contexts_request_id.saturating_add(1);
        let request_id = self.screener.contexts_request_id;
        let requested_symbols = symbols.clone();
        self.screener.contexts_request_symbols = requested_symbols.clone();
        self.screener.contexts_refresh_pending = false;
        self.screener.contexts_loading = true;
        clear_screener_status_if(&mut self.screener.status, |message| {
            message == SCREENER_LOADING_SYMBOLS_STATUS
                || message.starts_with(SCREENER_CONTEXT_FAILURE_PREFIX)
        });
        Task::perform(api::fetch_watchlist_contexts(symbols), move |result| {
            Message::ScreenerContextsLoaded(request_id, requested_symbols.clone(), now_ms, result)
        })
    }

    pub(crate) fn request_screener_history_refresh(&mut self) -> Task<Message> {
        self.request_screener_history_refresh_with_force(false)
    }

    fn request_screener_history_refresh_with_force(&mut self, force: bool) -> Task<Message> {
        let force = force || self.screener.history_refresh_pending;
        if self.screener.window_id.is_none() {
            self.screener.invalidate_history_refresh();
            return Task::none();
        }

        if self.screener_symbol_keys().is_empty() {
            self.screener.history.clear();
            self.screener.history_loaded_at.clear();
            self.screener.invalidate_history_refresh();
            self.screener.history_last_fetch_ms = None;
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let recently_refreshed = self
            .screener
            .history_last_fetch_ms
            .is_some_and(|last_fetch| {
                now_ms.saturating_sub(last_fetch) < SCREENER_HISTORY_REFRESH_MS
            });

        let symbols = self.screener_history_symbol_keys(now_ms, force);
        if self.screener.history_loading {
            if force || symbols != self.screener.history_request_symbols {
                self.screener.history_refresh_pending = true;
            }
            return Task::none();
        }

        if self.screener.contexts_loading && self.screener.contexts.is_empty() {
            if force {
                self.screener.history_refresh_pending = true;
            }
            return Task::none();
        }

        if recently_refreshed && !force {
            return Task::none();
        }

        if symbols.is_empty() {
            return Task::none();
        }

        self.screener.history_request_id = self.screener.history_request_id.saturating_add(1);
        let request_id = self.screener.history_request_id;
        let requested_symbols = symbols.clone();
        self.screener.history_request_symbols = requested_symbols.clone();
        self.screener.history_refresh_pending = false;
        self.screener.history_loading = true;
        Task::perform(
            api::fetch_screener_history(symbols.clone()),
            move |result| {
                Message::ScreenerHistoryLoaded(
                    request_id,
                    requested_symbols.clone(),
                    now_ms,
                    result,
                )
            },
        )
    }

    fn apply_screener_contexts_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<HashMap<String, api::WatchlistContext>, String>,
    ) -> Task<Message> {
        if !self.screener.contexts_loading
            || request_id != self.screener.contexts_request_id
            || requested_symbols != self.screener.contexts_request_symbols
        {
            return Task::none();
        }

        let current_symbols: HashSet<_> = self.screener_symbol_keys().into_iter().collect();
        let requested_symbol_set: HashSet<_> = requested_symbols.into_iter().collect();
        let has_current_requested_symbol = current_symbols
            .iter()
            .any(|symbol| requested_symbol_set.contains(symbol));
        let preserved_contexts: HashMap<String, api::WatchlistContext> = self
            .screener
            .contexts
            .iter()
            .filter(|(symbol, _)| {
                current_symbols.contains(*symbol) && !requested_symbol_set.contains(*symbol)
            })
            .map(|(symbol, context)| (symbol.clone(), context.clone()))
            .collect();
        let contexts_refresh_pending = self.screener.contexts_refresh_pending;
        self.screener.contexts_refresh_pending = false;
        self.screener.contexts_request_symbols.clear();
        self.screener.contexts_loading = false;

        match result {
            Ok(contexts) => {
                let mut scoped_contexts = preserved_contexts;
                scoped_contexts.extend(contexts.into_iter().filter(|(symbol, _)| {
                    requested_symbol_set.contains(symbol) && current_symbols.contains(symbol)
                }));
                self.screener.contexts = scoped_contexts;
                self.screener.contexts_last_fetch_ms = Some(requested_at);
                clear_screener_status_if(&mut self.screener.status, |message| {
                    message == SCREENER_LOADING_SYMBOLS_STATUS
                        || message.starts_with(SCREENER_CONTEXT_FAILURE_PREFIX)
                });
            }
            Err(error) if has_current_requested_symbol => {
                self.screener
                    .contexts
                    .retain(|symbol, _| current_symbols.contains(symbol));
                self.screener.status = Some((format!("Screener refresh failed: {error}"), true));
            }
            Err(_) => {
                self.screener.contexts = preserved_contexts;
            }
        }

        let context_task = if contexts_refresh_pending {
            self.request_screener_context_refresh(true)
        } else {
            Task::none()
        };
        Task::batch([context_task, self.request_screener_history_refresh()])
    }

    fn apply_screener_history_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<HashMap<String, (f64, f64)>, String>,
    ) -> Task<Message> {
        if !self.screener.history_loading
            || request_id != self.screener.history_request_id
            || requested_symbols != self.screener.history_request_symbols
        {
            return Task::none();
        }

        let current_symbols: HashSet<_> = self.screener_symbol_keys().into_iter().collect();
        let scoped_requested_symbols: Vec<_> = requested_symbols
            .into_iter()
            .filter(|symbol| {
                current_symbols.contains(symbol)
                    && self
                        .screener
                        .history_loaded_at
                        .get(symbol)
                        .is_none_or(|last| requested_at >= *last)
            })
            .collect();
        let scoped_requested_symbol_set: HashSet<_> =
            scoped_requested_symbols.iter().cloned().collect();
        let history_refresh_pending = self.screener.history_refresh_pending;
        self.screener.history_refresh_pending = false;
        self.screener.history_request_symbols.clear();
        self.screener.history_loading = false;
        self.screener
            .history
            .retain(|symbol, _| current_symbols.contains(symbol));
        self.screener
            .history_loaded_at
            .retain(|symbol, _| current_symbols.contains(symbol));

        match result {
            Ok(history) => {
                for symbol in scoped_requested_symbols {
                    self.screener.history.remove(&symbol);
                    self.screener.history_loaded_at.insert(symbol, requested_at);
                }
                self.screener.history.extend(
                    history
                        .into_iter()
                        .filter(|(symbol, _)| scoped_requested_symbol_set.contains(symbol)),
                );
                if !scoped_requested_symbol_set.is_empty() {
                    self.screener.history_last_fetch_ms = Some(requested_at);
                }
                clear_screener_status_if(&mut self.screener.status, |message| {
                    message.starts_with(SCREENER_HISTORY_FAILURE_PREFIX)
                });
            }
            Err(error) if !scoped_requested_symbol_set.is_empty() => {
                self.screener.status =
                    Some((format!("Screener history refresh failed: {error}"), true));
            }
            Err(_) => {}
        }

        if history_refresh_pending {
            self.request_screener_history_refresh_with_force(true)
        } else {
            Task::none()
        }
    }
}

fn clear_screener_status_if(
    status: &mut Option<(String, bool)>,
    predicate: impl FnOnce(&str) -> bool,
) {
    if status
        .as_ref()
        .is_some_and(|(message, _)| predicate(message))
    {
        *status = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType, WatchlistContext};
    use crate::screener_state::ScreenerExchangeFilter;

    fn context(day_vlm: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: Some(day_vlm),
        }
    }

    fn terminal_with_screener(symbols: &[&str]) -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.screener.window_id = Some(window::Id::unique());
        terminal.exchange_symbols = symbols.iter().map(|symbol| perp_symbol(symbol)).collect();
        terminal.screener.contexts.clear();
        terminal.screener.contexts_last_fetch_ms = None;
        terminal.screener.history.clear();
        terminal.screener.history_loaded_at.clear();
        terminal.screener.history_last_fetch_ms = None;
        terminal.screener.status = None;
        terminal.screener.invalidate_refreshes();
        terminal
    }

    fn perp_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key
                .split_once(':')
                .map(|(_, ticker)| ticker)
                .unwrap_or(key)
                .to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: Some(0),
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    #[test]
    fn screener_context_scope_change_queues_current_scope_after_in_flight_result() {
        let mut terminal = terminal_with_screener(&["BTC", "xyz:ETH"]);

        let _task = terminal.request_screener_context_refresh(true);
        let stale_request_id = terminal.screener.contexts_request_id;
        let stale_symbols = terminal.screener.contexts_request_symbols.clone();
        assert!(terminal.screener.contexts_loading);

        let _task = terminal.update_screener(Message::ScreenerExchangeFilterChanged(
            ScreenerExchangeFilter::AllHip3,
        ));
        assert!(terminal.screener.contexts_refresh_pending);

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            stale_request_id,
            stale_symbols,
            10,
            Ok(HashMap::from([
                ("BTC".to_string(), context(1.0)),
                ("xyz:ETH".to_string(), context(2.0)),
            ])),
        ));

        assert!(
            terminal.screener.contexts_loading,
            "queued refresh should start for the current screener scope"
        );
        assert!(!terminal.screener.contexts_refresh_pending);
        assert_eq!(
            terminal.screener.contexts_request_symbols,
            vec!["xyz:ETH".to_string()]
        );
        assert!(!terminal.screener.contexts.contains_key("BTC"));
        assert!(terminal.screener.contexts.contains_key("xyz:ETH"));
    }

    #[test]
    fn stale_screener_context_result_does_not_clear_current_request() {
        let mut terminal = terminal_with_screener(&["xyz:ETH"]);
        terminal.screener.contexts_loading = true;
        terminal.screener.contexts_request_id = 2;
        terminal.screener.contexts_request_symbols = vec!["xyz:ETH".to_string()];

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            1,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), context(1.0))])),
        ));

        assert!(terminal.screener.contexts_loading);
        assert_eq!(terminal.screener.contexts_request_id, 2);
        assert_eq!(
            terminal.screener.contexts_request_symbols,
            vec!["xyz:ETH".to_string()]
        );
        assert!(terminal.screener.contexts.is_empty());
        assert_eq!(terminal.screener.contexts_last_fetch_ms, None);
    }

    #[test]
    fn screener_context_result_keeps_only_current_nonrequested_and_loaded_symbols() {
        let mut terminal = terminal_with_screener(&["BTC", "xyz:ETH"]);
        terminal
            .screener
            .contexts
            .insert("BTC".to_string(), context(1.0));
        terminal
            .screener
            .contexts
            .insert("DOGE".to_string(), context(4.0));
        terminal.screener.exchange_filter = ScreenerExchangeFilter::AllHip3;
        terminal.screener.contexts_loading = true;
        terminal.screener.contexts_request_id = 7;
        terminal.screener.contexts_request_symbols = vec!["xyz:ETH".to_string()];

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            7,
            vec!["xyz:ETH".to_string()],
            10,
            Ok(HashMap::from([
                ("xyz:ETH".to_string(), context(2.0)),
                ("DOGE".to_string(), context(3.0)),
            ])),
        ));

        assert_eq!(
            terminal
                .screener
                .contexts
                .get("xyz:ETH")
                .and_then(|ctx| ctx.day_vlm),
            Some(2.0)
        );
        assert!(!terminal.screener.contexts.contains_key("BTC"));
        assert!(!terminal.screener.contexts.contains_key("DOGE"));
    }

    #[test]
    fn screener_context_success_clears_stale_omitted_requested_symbol() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal
            .screener
            .contexts
            .insert("BTC".to_string(), context(1.0));
        terminal.screener.contexts_loading = true;
        terminal.screener.contexts_request_id = 7;
        terminal.screener.contexts_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::new()),
        ));

        assert!(!terminal.screener.contexts_loading);
        assert_eq!(terminal.screener.contexts_last_fetch_ms, Some(10));
        assert!(!terminal.screener.contexts.contains_key("BTC"));
    }

    #[test]
    fn screener_context_error_keeps_current_contexts_without_marking_fresh() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal.screener.contexts_last_fetch_ms = Some(10);
        terminal
            .screener
            .contexts
            .insert("BTC".to_string(), context(1.0));
        terminal.screener.contexts_loading = true;
        terminal.screener.contexts_request_id = 7;
        terminal.screener.contexts_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            7,
            vec!["BTC".to_string()],
            20,
            Err("network".to_string()),
        ));

        assert!(!terminal.screener.contexts_loading);
        assert_eq!(terminal.screener.contexts_last_fetch_ms, Some(10));
        assert_eq!(
            terminal
                .screener
                .contexts
                .get("BTC")
                .and_then(|ctx| ctx.day_vlm),
            Some(1.0)
        );
        assert_eq!(
            terminal
                .screener
                .status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Screener refresh failed: network", true))
        );
    }

    #[test]
    fn screener_context_success_does_not_clear_history_failure_status() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal.screener.status =
            Some(("Screener history refresh failed: network".to_string(), true));
        terminal.screener.contexts_loading = true;
        terminal.screener.contexts_request_id = 7;
        terminal.screener.contexts_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), context(1.0))])),
        ));

        assert_eq!(
            terminal
                .screener
                .status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Screener history refresh failed: network", true))
        );
    }

    #[test]
    fn empty_screener_scope_clears_stale_status() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal.screener.status = Some(("Screener refresh failed: network".to_string(), true));
        terminal
            .screener
            .contexts
            .insert("BTC".to_string(), context(1.0));
        terminal.exchange_symbols.clear();

        let _task = terminal.request_screener_context_refresh(true);

        assert!(terminal.screener.status.is_none());
        assert!(terminal.screener.contexts.is_empty());
        assert_eq!(terminal.screener.contexts_last_fetch_ms, None);
    }

    #[test]
    fn empty_screener_scope_invalidates_in_flight_context_result() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal
            .screener
            .contexts
            .insert("BTC".to_string(), context(1.0));

        let _task = terminal.request_screener_context_refresh(true);
        let stale_request_id = terminal.screener.contexts_request_id;
        terminal.exchange_symbols.clear();
        let _task = terminal.request_screener_context_refresh(true);

        assert!(!terminal.screener.contexts_loading);
        assert!(terminal.screener.contexts_request_symbols.is_empty());
        assert!(terminal.screener.contexts.is_empty());
        assert_eq!(terminal.screener.contexts_last_fetch_ms, None);

        let _task = terminal.update_screener(Message::ScreenerContextsLoaded(
            stale_request_id,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), context(2.0))])),
        ));

        assert!(terminal.screener.contexts.is_empty());
        assert_eq!(terminal.screener.contexts_last_fetch_ms, None);
    }

    #[test]
    fn screener_history_error_with_scope_change_queues_current_retry() {
        let mut terminal = terminal_with_screener(&["BTC", "xyz:ETH"]);

        let _task = terminal.request_screener_history_refresh();
        let stale_request_id = terminal.screener.history_request_id;
        let stale_symbols = terminal.screener.history_request_symbols.clone();
        assert!(terminal.screener.history_loading);

        let _task = terminal.update_screener(Message::ScreenerExchangeFilterChanged(
            ScreenerExchangeFilter::AllHip3,
        ));
        assert!(terminal.screener.history_refresh_pending);

        terminal.screener.contexts_loading = false;
        let _task = terminal.update_screener(Message::ScreenerHistoryLoaded(
            stale_request_id,
            stale_symbols,
            10,
            Err("temporary failure".to_string()),
        ));

        assert!(
            terminal.screener.history_loading,
            "queued history retry should start for the current screener scope"
        );
        assert!(!terminal.screener.history_refresh_pending);
        assert_eq!(
            terminal.screener.history_request_symbols,
            vec!["xyz:ETH".to_string()]
        );
    }

    #[test]
    fn manual_screener_refresh_requests_history_for_loaded_symbol() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal
            .screener
            .contexts
            .insert("BTC".to_string(), context(1.0));
        terminal
            .screener
            .history
            .insert("BTC".to_string(), (1.0, 2.0));
        terminal
            .screener
            .history_loaded_at
            .insert("BTC".to_string(), 5);
        terminal.screener.history_last_fetch_ms = Some(TradingTerminal::now_ms());

        let _task = terminal.update_screener(Message::ForceRefreshScreener);

        assert!(terminal.screener.history_loading);
        assert_eq!(
            terminal.screener.history_request_symbols,
            vec!["BTC".to_string()]
        );
    }

    #[test]
    fn screener_history_result_filters_loaded_markers_and_payload_to_current_scope() {
        let mut terminal = terminal_with_screener(&["BTC", "xyz:ETH"]);
        terminal.screener.exchange_filter = ScreenerExchangeFilter::AllHip3;
        terminal
            .screener
            .history
            .insert("DOGE".to_string(), (7.0, 8.0));
        terminal
            .screener
            .history_loaded_at
            .insert("DOGE".to_string(), 5);
        terminal.screener.history_loading = true;
        terminal.screener.history_request_id = 7;
        terminal.screener.history_request_symbols = vec!["BTC".to_string(), "xyz:ETH".to_string()];

        let _task = terminal.update_screener(Message::ScreenerHistoryLoaded(
            7,
            vec!["BTC".to_string(), "xyz:ETH".to_string()],
            10,
            Ok(HashMap::from([
                ("BTC".to_string(), (1.0, 2.0)),
                ("xyz:ETH".to_string(), (3.0, 4.0)),
                ("DOGE".to_string(), (5.0, 6.0)),
            ])),
        ));

        assert!(!terminal.screener.history.contains_key("BTC"));
        assert_eq!(terminal.screener.history.get("xyz:ETH"), Some(&(3.0, 4.0)));
        assert!(!terminal.screener.history.contains_key("DOGE"));
        assert!(!terminal.screener.history_loaded_at.contains_key("BTC"));
        assert!(!terminal.screener.history_loaded_at.contains_key("DOGE"));
        assert_eq!(
            terminal.screener.history_loaded_at.get("xyz:ETH"),
            Some(&10)
        );
        assert_eq!(terminal.screener.history_last_fetch_ms, Some(10));
    }

    #[test]
    fn screener_history_success_removes_stale_requested_symbol_when_payload_omits_it() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal
            .screener
            .history
            .insert("BTC".to_string(), (1.0, 2.0));
        terminal
            .screener
            .history_loaded_at
            .insert("BTC".to_string(), 5);
        terminal.screener.history_loading = true;
        terminal.screener.history_request_id = 7;
        terminal.screener.history_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_screener(Message::ScreenerHistoryLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::new()),
        ));

        assert!(!terminal.screener.history_loading);
        assert!(!terminal.screener.history.contains_key("BTC"));
        assert_eq!(terminal.screener.history_loaded_at.get("BTC"), Some(&10));
        assert_eq!(terminal.screener.history_last_fetch_ms, Some(10));
    }

    #[test]
    fn screener_history_error_keeps_existing_history_without_marking_fresh() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal.screener.history_last_fetch_ms = Some(5);
        terminal
            .screener
            .history
            .insert("BTC".to_string(), (1.0, 2.0));
        terminal
            .screener
            .history_loaded_at
            .insert("BTC".to_string(), 5);
        terminal.screener.history_loading = true;
        terminal.screener.history_request_id = 7;
        terminal.screener.history_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_screener(Message::ScreenerHistoryLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Err("network".to_string()),
        ));

        assert!(!terminal.screener.history_loading);
        assert_eq!(terminal.screener.history.get("BTC"), Some(&(1.0, 2.0)));
        assert_eq!(terminal.screener.history_loaded_at.get("BTC"), Some(&5));
        assert_eq!(terminal.screener.history_last_fetch_ms, Some(5));
        assert_eq!(
            terminal
                .screener
                .status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Screener history refresh failed: network", true))
        );
    }

    #[test]
    fn screener_history_success_does_not_clear_context_failure_status() {
        let mut terminal = terminal_with_screener(&["BTC"]);
        terminal.screener.status = Some(("Screener refresh failed: network".to_string(), true));
        terminal.screener.history_loading = true;
        terminal.screener.history_request_id = 7;
        terminal.screener.history_request_symbols = vec!["BTC".to_string()];

        let _task = terminal.update_screener(Message::ScreenerHistoryLoaded(
            7,
            vec!["BTC".to_string()],
            10,
            Ok(HashMap::from([("BTC".to_string(), (1.0, 2.0))])),
        ));

        assert_eq!(
            terminal
                .screener
                .status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Screener refresh failed: network", true))
        );
    }
}
