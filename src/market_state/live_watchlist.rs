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
            self.is_ticker_muted(symbol)
        })
    }

    pub(crate) fn request_live_watchlist_refresh(&mut self, force: bool) -> Task<Message> {
        let symbols = self.watched_live_watchlist_symbols();
        let plan = plan_live_watchlist_refresh(LiveWatchlistRefreshInput {
            symbols,
            force,
            now_ms: Self::now_ms(),
            contexts_last_fetch_ms: self.live_watchlist_contexts_last_fetch_ms,
            contexts: &self.live_watchlist_ctxs,
            contexts_loading: self.live_watchlist_contexts_loading,
            history_loaded_at: &self.live_watchlist_history_loaded_at,
            history_loading: self.live_watchlist_history_loading,
        });
        if !plan.has_requests() {
            return Task::none();
        }

        let mut tasks = Vec::new();

        if !plan.context_symbols.is_empty() {
            self.live_watchlist_contexts_loading = true;
            let requested_at = plan.requested_at;
            tasks.push(Task::perform(
                api::fetch_watchlist_contexts(plan.context_symbols),
                move |result| Message::LiveWatchlistContextsLoaded(requested_at, result),
            ));
        }

        if !plan.history_symbols.is_empty() {
            self.live_watchlist_history_loading = true;
            let requested_symbols = plan.history_symbols.clone();
            let requested_at = plan.requested_at;
            tasks.push(Task::perform(
                api::fetch_watchlist_history(plan.history_symbols),
                move |result| {
                    Message::LiveWatchlistHistoryLoaded(
                        requested_symbols.clone(),
                        requested_at,
                        result,
                    )
                },
            ));
        }

        self.live_watchlist_status = None;
        Task::batch(tasks)
    }
}
