use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::market_state::{LiveWatchlistId, LiveWatchlistInstance};
use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    pub(super) fn boot_live_watchlists(
        cfg: &KeroseneConfig,
        muted_tickers: &HashSet<String>,
    ) -> HashMap<LiveWatchlistId, LiveWatchlistInstance> {
        cfg.live_watchlists
            .clone()
            .into_iter()
            .map(|watchlist_config| {
                (
                    watchlist_config.id,
                    LiveWatchlistInstance {
                        id: watchlist_config.id,
                        symbols: watchlist_config
                            .symbols
                            .into_iter()
                            .filter(|symbol| {
                                !Self::key_matches_muted_tickers(&[], muted_tickers, symbol)
                            })
                            .collect(),
                        search_query: String::new(),
                        sort_column: watchlist_config.sort_column,
                        sort_direction: watchlist_config.sort_direction,
                        visible_columns: watchlist_config.visible_columns,
                        row_cache: Vec::new(),
                    },
                )
            })
            .collect()
    }
}
