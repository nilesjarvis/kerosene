mod controls;
mod panes;
mod results;
mod symbols;

use super::context_results::scope_context_response;
use crate::app_state::TradingTerminal;
use crate::message::Message;
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
            Message::AddLiveWatchlistPane => self.add_live_watchlist_pane(),
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
                let preset_id = self
                    .live_watchlists
                    .get(&id)
                    .and_then(|watchlist| watchlist.preset_id);
                if let Some(preset_id) = preset_id {
                    let is_new = self
                        .watchlist_preset(preset_id)
                        .is_some_and(|preset| !preset.symbols.contains(&symbol));
                    if is_new && let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                        watchlist.search_query.clear();
                    }
                    return self.add_watchlist_preset_symbol(preset_id, symbol);
                }
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    add_watchlist_symbol(watchlist, symbol);
                }
                self.refresh_live_watchlist_row_cache(id);
                self.persist_config();
                self.request_live_watchlist_refresh(true)
            }
            Message::LiveWatchlistRemoveSymbol(id, symbol) => {
                let preset_id = self
                    .live_watchlists
                    .get(&id)
                    .and_then(|watchlist| watchlist.preset_id);
                if let Some(preset_id) = preset_id {
                    return self.remove_watchlist_preset_symbol(preset_id, &symbol);
                }
                if let Some(watchlist) = self.live_watchlists.get_mut(&id) {
                    remove_watchlist_symbol(watchlist, &symbol);
                }
                self.refresh_live_watchlist_row_cache(id);
                self.persist_config();
                Task::none()
            }
            Message::LiveWatchlistPresetSelected(id, preset_id) => {
                self.select_live_watchlist_preset(id, preset_id)
            }
            Message::LiveWatchlistCreatePreset(id) => self.create_live_watchlist_preset(id),
            Message::WatchlistPresetNameChanged(preset_id, name) => {
                self.rename_watchlist_preset(preset_id, name)
            }
            Message::WatchlistPresetDelete(preset_id) => self.delete_watchlist_preset(preset_id),
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
                result,
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
                result,
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
        let scoped_result = scope_context_response(
            &mut self.live_watchlist_ctxs,
            &current_symbols,
            requested_symbols,
            result,
        );
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
mod tests;
