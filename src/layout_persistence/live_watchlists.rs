use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::LiveWatchlistInstance;

// ---------------------------------------------------------------------------
// Layout Live-Watchlist Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_live_watchlists(&mut self, layout: &config::SavedLayout) {
        self.live_watchlists = layout
            .live_watchlists
            .clone()
            .into_iter()
            .map(|watchlist_config| {
                (
                    watchlist_config.id,
                    LiveWatchlistInstance {
                        id: watchlist_config.id,
                        symbols: {
                            let mut seen = std::collections::HashSet::new();
                            watchlist_config
                                .symbols
                                .into_iter()
                                .filter(|symbol| !self.symbol_key_is_hidden(symbol))
                                .map(|symbol| {
                                    self.exchange_symbol_for_key(&symbol)
                                        .map(|metadata| metadata.key.clone())
                                        .unwrap_or(symbol)
                                })
                                .filter(|symbol| seen.insert(symbol.clone()))
                                .collect()
                        },
                        search_query: String::new(),
                        sort_column: watchlist_config.sort_column,
                        sort_direction: watchlist_config.sort_direction,
                        visible_columns: watchlist_config.visible_columns,
                        row_cache: Vec::new(),
                    },
                )
            })
            .collect();
        self.refresh_live_watchlist_row_caches();
    }
}
