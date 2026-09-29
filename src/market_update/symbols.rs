mod charts;
mod contexts;
mod controls;
mod migration;
mod outcome_volumes;
mod refresh;
mod resolution;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

use self::controls::{apply_hip3_dex_filter, apply_market_filter, toggle_favourite_symbol};

impl TradingTerminal {
    pub(super) fn update_symbol_search_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleFavourite(key) => self.toggle_market_favourite(key),
            Message::SymbolsLoaded(result) => self.apply_symbols_loaded(result),
            Message::ExchangeSymbolsRefreshTick => self.request_exchange_symbols_refresh(),
            Message::SymbolSearchChanged(query) => {
                self.symbol_search_query = query;
                self.refresh_symbol_search_results();
                Task::none()
            }
            Message::SymbolSearchSortChanged(sort_mode) => {
                self.symbol_search_sort_mode = sort_mode;
                self.refresh_symbol_search_results();
                self.persist_config();
                self.request_symbol_search_context_refresh(false)
            }
            Message::SymbolSearchMarketFilterChanged(filter) => {
                apply_market_filter(
                    &mut self.symbol_search_market_filter,
                    &mut self.symbol_search_hip3_dex_filter,
                    filter,
                );
                self.refresh_symbol_search_results();
                self.request_symbol_search_context_refresh(false)
            }
            Message::SymbolSearchHip3DexFilterChanged(dex) => {
                apply_hip3_dex_filter(&mut self.symbol_search_hip3_dex_filter, dex);
                self.refresh_symbol_search_results();
                self.request_symbol_search_context_refresh(false)
            }
            Message::SymbolSearchContextsLoaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ) => self.apply_symbol_search_contexts_loaded(
                request_id,
                requested_symbols,
                requested_at,
                result,
            ),
            Message::OutcomeSearchChanged(query) => {
                self.outcome_search_query = query;
                Task::none()
            }
            Message::OutcomeVenueFilterChanged(venue) => {
                self.outcome_venue_filter = venue;
                Task::none()
            }
            Message::OutcomeRulesToggled(outcome_id) => {
                if !self.outcome_expanded_rules.insert(outcome_id) {
                    self.outcome_expanded_rules.remove(&outcome_id);
                }
                Task::none()
            }
            Message::OutcomeMarketGroupToggled(key) => {
                if !self.outcome_collapsed_market_groups.insert(key.clone()) {
                    self.outcome_collapsed_market_groups.remove(&key);
                }
                Task::none()
            }
            Message::OutcomeVolumesLoaded(request_id, requested_symbols, result) => {
                self.apply_outcome_volumes_loaded(request_id, requested_symbols, result)
            }
            Message::SymbolSelected(key) => self.select_market_symbol(key),
            _ => Task::none(),
        }
    }

    fn toggle_market_favourite(&mut self, key: String) -> Task<Message> {
        if self.symbol_key_is_hidden(&key) {
            let display = self.display_name_for_symbol(&key);
            self.symbol_search_status =
                Some((format!("{display} is hidden by Settings > Risk"), true));
            return Task::none();
        }
        toggle_favourite_symbol(&mut self.favourite_symbols, key);
        self.refresh_symbol_search_results();
        self.persist_config();
        self.request_ticker_tape_context_refresh(true)
    }

    fn select_market_symbol(&mut self, key: String) -> Task<Message> {
        if self.active_symbol == key {
            return Task::none();
        }

        self.switch_active_symbol_internal(key)
    }
}

#[cfg(test)]
mod tests;
