use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;
use std::collections::{BTreeSet, HashMap};

// ---------------------------------------------------------------------------
// Ticker Tape Context Refresh
// ---------------------------------------------------------------------------

const TICKER_TAPE_CONTEXT_REFRESH_MS: u64 = 300_000;

impl TradingTerminal {
    pub(crate) fn request_ticker_tape_context_refresh(&mut self, force: bool) -> Task<Message> {
        if !self.ticker_tape_enabled {
            self.ticker_tape_contexts_loading = false;
            return Task::none();
        }

        let symbols = self.ticker_tape_context_symbols();
        if symbols.is_empty() {
            self.ticker_tape_ctxs.clear();
            self.ticker_tape_contexts_loading = false;
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

        if self.ticker_tape_contexts_loading || (!force && !contexts_stale && !contexts_missing) {
            return Task::none();
        }

        self.ticker_tape_contexts_loading = true;
        Task::perform(api::fetch_watchlist_contexts(symbols), move |result| {
            Message::TickerTapeContextsLoaded(now_ms, result)
        })
    }

    pub(super) fn update_ticker_tape_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TickerTapeRefreshTick => self.request_ticker_tape_context_refresh(false),
            Message::TickerTapeContextsLoaded(requested_at, result) => {
                self.apply_ticker_tape_contexts_loaded(requested_at, result);
                Task::none()
            }
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
        requested_at: u64,
        result: Result<HashMap<String, api::WatchlistContext>, String>,
    ) {
        if self
            .ticker_tape_contexts_last_fetch_ms
            .is_some_and(|last| requested_at < last)
        {
            return;
        }

        self.ticker_tape_contexts_loading = false;

        if let Ok(contexts) = result {
            self.ticker_tape_contexts_last_fetch_ms = Some(requested_at);
            self.ticker_tape_ctxs = contexts;
        }
    }
}
