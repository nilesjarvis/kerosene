mod contexts;
mod markets;
mod results;
mod symbols;
mod volume;

use super::types::SymbolSearchSortMode;
use crate::api::{self, ExchangeSymbol};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use self::contexts::{SymbolSearchContextRefreshInput, plan_context_refresh};
use self::markets::{
    symbol_search_exchange_label, symbol_search_hip3_dexes, symbol_search_matches_market_filter,
};
use self::results::{SymbolSearchResultsInput, filtered_symbol_search_indices};
use self::symbols::context_symbol_keys;
use self::volume::{format_symbol_search_volume, symbol_search_volume};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn symbol_search_hip3_dexes(&self) -> Vec<String> {
        symbol_search_hip3_dexes(&self.exchange_symbols)
    }

    pub(crate) fn symbol_search_matches_market_filter(&self, symbol: &ExchangeSymbol) -> bool {
        symbol_search_matches_market_filter(
            symbol,
            self.symbol_search_market_filter,
            self.symbol_search_hip3_dex_filter.as_deref(),
        )
    }

    pub(crate) fn symbol_search_context_symbols(&self) -> Vec<String> {
        context_symbol_keys(
            self.exchange_symbols.iter(),
            |symbol| self.symbol_search_matches_market_filter(symbol),
            |symbol| self.exchange_symbol_is_hidden(symbol),
        )
    }

    pub(crate) fn refresh_symbol_search_results(&mut self) {
        let (indices, favourite_count) = filtered_symbol_search_indices(SymbolSearchResultsInput {
            symbols: &self.exchange_symbols,
            query: &self.symbol_search_query,
            sort_mode: self.symbol_search_sort_mode,
            market_filter: self.symbol_search_market_filter,
            hip3_dex_filter: self.symbol_search_hip3_dex_filter.as_deref(),
            favourite_symbols: &self.favourite_symbols,
            contexts: &self.symbol_search_ctxs,
            is_muted: |symbol| self.exchange_symbol_is_hidden(symbol),
        });
        self.symbol_search_result_indices = indices;
        self.symbol_search_favourite_count = favourite_count;
    }

    pub(crate) fn request_symbol_search_context_refresh(&mut self, force: bool) -> Task<Message> {
        let symbols = self.symbol_search_context_symbols();
        let Some(plan) = plan_context_refresh(SymbolSearchContextRefreshInput {
            symbols,
            force,
            now_ms: Self::now_ms(),
            sort_mode: self.symbol_search_sort_mode,
            contexts_loading: self.symbol_search_contexts_loading,
            contexts_last_fetch_ms: self.symbol_search_contexts_last_fetch_ms,
            contexts: &self.symbol_search_ctxs,
        }) else {
            return Task::none();
        };

        self.symbol_search_contexts_loading = true;
        self.symbol_search_status = None;
        let requested_at = plan.requested_at;
        Task::perform(api::fetch_watchlist_contexts(plan.symbols), move |result| {
            Message::SymbolSearchContextsLoaded(requested_at, result)
        })
    }

    pub(crate) fn symbol_search_exchange_label(symbol: &ExchangeSymbol) -> String {
        symbol_search_exchange_label(symbol)
    }

    pub(crate) fn symbol_search_volume(&self, symbol: &ExchangeSymbol) -> Option<f64> {
        symbol_search_volume(&self.symbol_search_ctxs, symbol)
    }

    pub(crate) fn format_symbol_search_volume(value: f64) -> String {
        format_symbol_search_volume(value)
    }
}
