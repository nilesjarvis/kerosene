use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{OrderBookDisplayMode, OrderBookSymbolMode};

impl TradingTerminal {
    pub(crate) fn order_book_configs_snapshot(&self) -> Vec<config::OrderBookConfig> {
        self.order_books
            .values()
            .map(|book| config::OrderBookConfig {
                id: book.id,
                mode: match &book.mode {
                    OrderBookSymbolMode::Active => config::OrderBookSymbolModeConfig::Active,
                    OrderBookSymbolMode::Fixed(symbol) => {
                        if self.symbol_key_is_hidden(symbol) {
                            config::OrderBookSymbolModeConfig::Active
                        } else {
                            config::OrderBookSymbolModeConfig::Fixed(symbol.clone())
                        }
                    }
                },
                tick_size: Self::normalized_book_tick_size(book.tick_size),
                display_mode: match book.display_mode {
                    OrderBookDisplayMode::DepthList => {
                        config::OrderBookDisplayModeConfig::DepthList
                    }
                    OrderBookDisplayMode::DomLadder => {
                        config::OrderBookDisplayModeConfig::DomLadder
                    }
                    OrderBookDisplayMode::DepthChart => {
                        config::OrderBookDisplayModeConfig::DepthChart
                    }
                },
                center_on_mid: book.center_on_mid,
                reverse_side: book.reverse_side,
                show_spread_chart: book.show_spread_chart,
                spread_chart_height: book.spread_chart_height,
            })
            .collect()
    }

    pub(crate) fn live_watchlist_configs_snapshot(&self) -> Vec<config::LiveWatchlistConfig> {
        self.live_watchlists
            .values()
            .map(|watchlist| config::LiveWatchlistConfig {
                id: watchlist.id,
                symbols: watchlist
                    .symbols
                    .iter()
                    .filter(|symbol| !self.symbol_key_is_hidden(symbol))
                    .cloned()
                    .collect(),
                sort_column: watchlist.sort_column,
                sort_direction: watchlist.sort_direction,
                visible_columns: watchlist.visible_columns.clone(),
            })
            .collect()
    }

    pub(crate) fn positioning_info_configs_snapshot(&self) -> Vec<config::PositioningInfoConfig> {
        let mut instances: Vec<_> = self.positioning_infos.values().collect();
        instances.sort_by_key(|instance| instance.id);
        instances
            .into_iter()
            .map(|instance| config::PositioningInfoConfig {
                id: instance.id,
                page: instance.page,
                symbol: if self.symbol_key_is_hidden(&instance.symbol) {
                    self.fallback_unmuted_symbol_key().unwrap_or_default()
                } else {
                    instance.symbol.clone()
                },
                side: instance.side,
                sort_field: instance.sort_field,
                sort_direction: instance.sort_direction,
                change_timeframe: instance.change_timeframe,
                change_sort_field: instance.change_sort_field,
                change_sort_direction: instance.change_sort_direction,
            })
            .collect()
    }

    pub(crate) fn session_data_configs_snapshot(&self) -> Vec<config::SessionDataConfig> {
        let mut instances: Vec<_> = self.session_data.values().collect();
        instances.sort_by_key(|instance| instance.id);
        instances
            .into_iter()
            .map(|instance| config::SessionDataConfig {
                id: instance.id,
                symbol: if self.symbol_key_is_hidden(&instance.symbol) {
                    self.fallback_unmuted_symbol_key().unwrap_or_default()
                } else {
                    instance.symbol.clone()
                },
                lookback: instance.lookback,
            })
            .collect()
    }
}
