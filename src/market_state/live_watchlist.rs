mod columns;
mod refresh;
mod rows;
mod symbols;

use super::types::LiveWatchlistId;
use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use self::refresh::{LiveWatchlistRefreshInput, plan_live_watchlist_refresh};
use self::symbols::{open_live_watchlist_ids, watched_symbol_keys};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn open_live_watchlist_ids(&self) -> std::collections::HashSet<LiveWatchlistId> {
        open_live_watchlist_ids(self.panes.iter().map(|(_, kind)| kind))
    }

    pub(crate) fn watched_live_watchlist_symbols(&self) -> Vec<String> {
        let open_ids = self.open_live_watchlist_ids();
        watched_symbol_keys(&self.live_watchlists, &open_ids, |symbol| {
            self.symbol_key_is_hidden(symbol)
        })
    }

    pub(crate) fn request_live_watchlist_refresh(&mut self, force: bool) -> Task<Message> {
        let mut symbols = self.watched_live_watchlist_symbols();
        if self.symbols_loading {
            // The canonical key for the legacy `@0` pair is only known after
            // spotMeta loads. Defer rather than issue failing raw candle calls.
            symbols.retain(|symbol| symbol != "@0");
        }
        let plan = plan_live_watchlist_refresh(LiveWatchlistRefreshInput {
            symbols,
            force,
            now_ms: Self::now_ms(),
            contexts_last_fetch_ms: self.live_watchlist_contexts_last_fetch_ms,
            contexts: &self.live_watchlist_ctxs,
            contexts_loading: false,
            history_loaded_at: &self.live_watchlist_history_loaded_at,
            history_loading: false,
        });
        if !plan.has_requests() {
            return Task::none();
        }

        let mut tasks = Vec::new();

        if !plan.context_symbols.is_empty() {
            if self.live_watchlist_contexts_loading {
                if plan.context_symbols != self.live_watchlist_contexts_request_symbols {
                    self.live_watchlist_contexts_refresh_pending = true;
                }
            } else {
                self.live_watchlist_contexts_request_id =
                    self.live_watchlist_contexts_request_id.saturating_add(1);
                let request_id = self.live_watchlist_contexts_request_id;
                let requested_symbols = plan.context_symbols.clone();
                self.live_watchlist_contexts_request_symbols = requested_symbols.clone();
                self.live_watchlist_contexts_refresh_pending = false;
                self.live_watchlist_contexts_loading = true;
                let requested_at = plan.requested_at;
                tasks.push(Task::perform(
                    api::fetch_watchlist_contexts(plan.context_symbols),
                    move |result| {
                        Message::LiveWatchlistContextsLoaded(
                            request_id,
                            requested_symbols.clone(),
                            requested_at,
                            result,
                        )
                    },
                ));
            }
        }

        if !plan.history_symbols.is_empty() {
            if self.live_watchlist_history_loading {
                if plan.history_symbols != self.live_watchlist_history_request_symbols {
                    self.live_watchlist_history_refresh_pending = true;
                }
            } else {
                self.live_watchlist_history_request_id =
                    self.live_watchlist_history_request_id.saturating_add(1);
                let request_id = self.live_watchlist_history_request_id;
                let requested_symbols = plan.history_symbols.clone();
                self.live_watchlist_history_request_symbols = requested_symbols.clone();
                self.live_watchlist_history_refresh_pending = false;
                self.live_watchlist_history_loading = true;
                let requested_at = plan.requested_at;
                tasks.push(Task::perform(
                    api::fetch_watchlist_history(plan.history_symbols),
                    move |result| {
                        Message::LiveWatchlistHistoryLoaded(
                            request_id,
                            requested_symbols.clone(),
                            requested_at,
                            result,
                        )
                    },
                ));
            }
        }

        if !tasks.is_empty() {
            self.live_watchlist_status = None;
        }
        Task::batch(tasks)
    }
}
