mod controls;
mod results;
mod symbols;

use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::LiveWatchlistInstance;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

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
            Message::LiveWatchlistContextsLoaded(requested_at, result) => {
                apply_contexts_loaded(
                    &mut self.live_watchlist_contexts_loading,
                    &mut self.live_watchlist_contexts_last_fetch_ms,
                    &mut self.live_watchlist_ctxs,
                    &mut self.live_watchlist_status,
                    requested_at,
                    result,
                );
                self.refresh_live_watchlist_row_caches();
                Task::none()
            }
            Message::LiveWatchlistHistoryLoaded(requested_symbols, requested_at, result) => {
                apply_history_loaded(
                    &mut self.live_watchlist_history_loading,
                    &mut self.live_watchlist_history_loaded_at,
                    &mut self.live_watchlist_history,
                    &mut self.live_watchlist_status,
                    requested_symbols,
                    requested_at,
                    result,
                );
                self.refresh_live_watchlist_row_caches();
                Task::none()
            }
            _ => Task::none(),
        }
    }
}
