use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::screener_state::{SCREENER_CONTEXT_REFRESH_MS, SCREENER_HISTORY_REFRESH_MS};

use iced::{Size, Task, window};

// ---------------------------------------------------------------------------
// Screener Update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_screener(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenScreenerWindow => self.open_screener_window(),
            Message::RefreshScreener => self.request_screener_data_refresh(false),
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
            Message::ScreenerContextsLoaded(requested_at, result) => {
                self.apply_screener_contexts_loaded(requested_at, result)
            }
            Message::ScreenerHistoryLoaded(requested_symbols, requested_at, result) => {
                self.apply_screener_history_loaded(requested_symbols, requested_at, result)
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
        self.force_record_screener_mid_samples(&mids, now_ms);

        Task::batch([
            task.map(Message::WindowOpened),
            self.request_screener_data_refresh(true),
        ])
    }

    pub(crate) fn request_screener_data_refresh(&mut self, force: bool) -> Task<Message> {
        let context_task = self.request_screener_context_refresh(force);
        let history_task = self.request_screener_history_refresh();
        Task::batch([context_task, history_task])
    }

    pub(crate) fn request_screener_context_refresh(&mut self, force: bool) -> Task<Message> {
        if self.screener.window_id.is_none() || self.screener.contexts_loading {
            return Task::none();
        }

        let symbols = self.screener_symbol_keys();
        if symbols.is_empty() {
            self.screener.status = Some(("Loading symbols".to_string(), false));
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
        if recently_refreshed && (!force || !missing_contexts) {
            return Task::none();
        }

        self.screener.contexts_loading = true;
        self.screener.status = None;
        Task::perform(api::fetch_watchlist_contexts(symbols), move |result| {
            Message::ScreenerContextsLoaded(now_ms, result)
        })
    }

    pub(crate) fn request_screener_history_refresh(&mut self) -> Task<Message> {
        if self.screener.window_id.is_none() || self.screener.history_loading {
            return Task::none();
        }
        if self.screener.contexts_loading && self.screener.contexts.is_empty() {
            return Task::none();
        }

        let now_ms = Self::now_ms();
        if self
            .screener
            .history_last_fetch_ms
            .is_some_and(|last_fetch| {
                now_ms.saturating_sub(last_fetch) < SCREENER_HISTORY_REFRESH_MS
            })
        {
            return Task::none();
        }

        let symbols = self.screener_history_symbol_keys(now_ms);
        if symbols.is_empty() {
            return Task::none();
        }

        self.screener.history_loading = true;
        self.screener.history_last_fetch_ms = Some(now_ms);
        Task::perform(
            api::fetch_screener_history(symbols.clone()),
            move |result| Message::ScreenerHistoryLoaded(symbols.clone(), now_ms, result),
        )
    }

    fn apply_screener_contexts_loaded(
        &mut self,
        requested_at: u64,
        result: Result<std::collections::HashMap<String, api::WatchlistContext>, String>,
    ) -> Task<Message> {
        self.screener.contexts_loading = false;
        self.screener.contexts_last_fetch_ms = Some(requested_at);

        match result {
            Ok(contexts) => {
                self.screener.contexts.extend(contexts);
                self.screener.status = None;
            }
            Err(error) => {
                self.screener.status = Some((format!("Screener refresh failed: {error}"), true));
            }
        }

        self.request_screener_history_refresh()
    }

    fn apply_screener_history_loaded(
        &mut self,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<std::collections::HashMap<String, (f64, f64)>, String>,
    ) -> Task<Message> {
        self.screener.history_loading = false;

        match result {
            Ok(history) => {
                for symbol in requested_symbols {
                    self.screener.history_loaded_at.insert(symbol, requested_at);
                }
                self.screener.history.extend(history);
                self.screener.status = None;
            }
            Err(error) => {
                self.screener.status =
                    Some((format!("Screener history refresh failed: {error}"), true));
            }
        }

        Task::none()
    }
}
