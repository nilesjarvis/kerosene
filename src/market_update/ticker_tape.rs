use super::context_results::scope_context_response;
use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Ticker Tape Context Refresh
// ---------------------------------------------------------------------------

const TICKER_TAPE_CONTEXT_REFRESH_MS: u64 = 300_000;
// A complete total queries every perp DEX plus spot. Keep the cadence below
// Hyperliquid's shared weighted REST limit while still updating continuously.
const TICKER_TAPE_EXCHANGE_STATS_REFRESH_MS: u64 = 60_000;

impl TradingTerminal {
    pub(crate) fn request_ticker_tape_refresh(&mut self, force: bool) -> Task<Message> {
        Task::batch([
            self.request_ticker_tape_context_refresh(force),
            self.request_ticker_tape_exchange_stats_refresh(force),
        ])
    }

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
            Message::TickerTapeRefreshTick => self.request_ticker_tape_refresh(false),
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
            Message::TickerTapeExchangeStatsLoaded(request_id, requested_at, result) => {
                self.apply_ticker_tape_exchange_stats_loaded(request_id, requested_at, result)
            }
            _ => Task::none(),
        }
    }

    fn request_ticker_tape_exchange_stats_refresh(&mut self, force: bool) -> Task<Message> {
        if !self.ticker_tape_enabled {
            self.invalidate_ticker_tape_exchange_stats_request();
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let stats_stale = self
            .ticker_tape_exchange_stats_last_fetch_ms
            .is_none_or(|last| {
                now_ms.saturating_sub(last) >= TICKER_TAPE_EXCHANGE_STATS_REFRESH_MS
            });
        if self.ticker_tape_exchange_stats_loading || (!force && !stats_stale) {
            return Task::none();
        }

        self.ticker_tape_exchange_stats_request_id =
            self.ticker_tape_exchange_stats_request_id.saturating_add(1);
        let request_id = self.ticker_tape_exchange_stats_request_id;
        self.ticker_tape_exchange_stats_loading = true;
        Task::perform(api::fetch_exchange_stats(), move |result| {
            Message::TickerTapeExchangeStatsLoaded(request_id, now_ms, result)
        })
    }

    fn apply_ticker_tape_exchange_stats_loaded(
        &mut self,
        request_id: u64,
        requested_at: u64,
        result: Result<api::ExchangeStats, String>,
    ) -> Task<Message> {
        if !self.ticker_tape_exchange_stats_loading
            || request_id != self.ticker_tape_exchange_stats_request_id
        {
            return Task::none();
        }

        self.ticker_tape_exchange_stats_loading = false;
        if let Ok(stats) = result {
            self.ticker_tape_exchange_stats = Some(stats);
            self.ticker_tape_exchange_stats_last_fetch_ms = Some(requested_at);
        }
        Task::none()
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

        let current_symbols = self.ticker_tape_context_symbols().into_iter().collect();
        let scoped_result = scope_context_response(
            &mut self.ticker_tape_ctxs,
            &current_symbols,
            requested_symbols,
            result,
        );
        let refresh_pending = self.ticker_tape_contexts_refresh_pending;
        self.ticker_tape_contexts_refresh_pending = false;
        self.ticker_tape_contexts_request_symbols.clear();
        self.ticker_tape_contexts_loading = false;

        if let Ok(response) = scoped_result {
            self.ticker_tape_contexts_last_fetch_ms = Some(requested_at);
            self.ticker_tape_ctxs = response.contexts;
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

    fn invalidate_ticker_tape_exchange_stats_request(&mut self) {
        self.ticker_tape_exchange_stats_request_id =
            self.ticker_tape_exchange_stats_request_id.saturating_add(1);
        self.ticker_tape_exchange_stats_loading = false;
    }
}

#[cfg(test)]
mod tests;
