use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{OrderBookDisplayMode, OrderBookSymbolMode};
use crate::signing::OrderKind;

// ---------------------------------------------------------------------------
// Widget Config Snapshots
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn chart_configs_snapshot(&self) -> Vec<config::ChartConfig> {
        let mut chart_instances: Vec<_> = self.charts.values().collect();
        chart_instances.sort_by_key(|inst| inst.id);
        chart_instances
            .into_iter()
            .map(|inst| config::ChartConfig {
                id: inst.id,
                symbol: if self.symbol_key_is_hidden(&inst.symbol) {
                    String::new()
                } else {
                    inst.symbol.clone()
                },
                timeframe: inst.interval.config_str().to_string(),
                annotations: inst
                    .annotations
                    .iter()
                    .filter(|annotation| annotation.is_valid())
                    .map(|annotation| annotation.to_config())
                    .collect(),
                inverted: inst.chart.inverted,
                show_trade_markers: inst.chart.show_trade_markers,
                funding_panel_height: inst.chart.funding_panel_height_config(),
                macro_indicators: inst.macro_indicators.clone(),
                open_interest_as_notional: inst.open_interest_as_notional,
            })
            .collect()
    }

    pub(crate) fn spaghetti_chart_configs_snapshot(&self) -> Vec<config::SpaghettiChartConfig> {
        let mut spaghetti_instances: Vec<_> = self.spaghetti_charts.values().collect();
        spaghetti_instances.sort_by_key(|inst| inst.id);
        spaghetti_instances
            .into_iter()
            .map(|inst| config::SpaghettiChartConfig {
                id: inst.id,
                symbols: inst
                    .canvas
                    .series
                    .iter()
                    .filter(|series| !self.symbol_key_is_hidden(&series.symbol))
                    .map(|series| series.symbol.clone())
                    .collect(),
                timeframe: inst.interval.config_str().to_string(),
                pair_mode: inst.pair_mode,
                pair_candle_mode: inst.pair_candle_mode,
                color_mode: inst.canvas.color_mode,
                show_labels: inst.canvas.show_labels,
                anchor: inst
                    .canvas
                    .active_session
                    .map(|session| session.config_str().to_string()),
                anchor_granularity: inst
                    .session_granularity
                    .map(|granularity| granularity.config_str().to_string()),
            })
            .collect()
    }

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
                tick_size: book.tick_size,
                display_mode: match book.display_mode {
                    OrderBookDisplayMode::DepthList => {
                        config::OrderBookDisplayModeConfig::DepthList
                    }
                    OrderBookDisplayMode::DomLadder => {
                        config::OrderBookDisplayModeConfig::DomLadder
                    }
                },
                center_on_mid: book.center_on_mid,
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

    pub(crate) fn active_symbol_config_value(&self) -> String {
        if self.symbol_key_is_hidden(&self.active_symbol) {
            self.fallback_unmuted_symbol_key().unwrap_or_default()
        } else {
            self.active_symbol.clone()
        }
    }

    pub(crate) fn active_timeframe_config_value(&self) -> String {
        self.primary_chart_id
            .and_then(|id| self.charts.get(&id))
            .map(|inst| inst.interval.config_str().to_string())
            .unwrap_or_else(|| "H1".to_string())
    }

    pub(crate) fn favourite_symbols_config_values(&self) -> Vec<String> {
        self.favourite_symbols
            .iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .cloned()
            .collect()
    }

    pub(crate) fn order_kind_config_value(&self) -> String {
        match self.order_kind {
            OrderKind::Market => "Market",
            OrderKind::Limit => "Limit",
            OrderKind::Chase => "Chase",
            OrderKind::LimitIoc => "Limit IOC",
        }
        .to_string()
    }

    pub(crate) fn saved_layout_snapshot(&self, name: String) -> config::SavedLayout {
        config::SavedLayout {
            name,
            pane_layout: self.collect_pane_layout(),
            layout_ratios: self.collect_layout_ratios(),
            charts: self.chart_configs_snapshot(),
            order_books: self.order_book_configs_snapshot(),
            live_watchlists: self.live_watchlist_configs_snapshot(),
            spaghetti_charts: self.spaghetti_chart_configs_snapshot(),
            active_symbol: self.active_symbol_config_value(),
            active_timeframe: self.active_timeframe_config_value(),
            order_kind: self.order_kind_config_value(),
            reduce_only: self.order_reduce_only,
            book_tick_size: 0.0,
            favourite_symbols: self.favourite_symbols_config_values(),
            active_theme: self.active_theme.clone(),
            custom_themes: self.custom_themes.clone(),
            sound_enabled: self.sound_enabled,
            desktop_notifications: self.desktop_notifications,
            income_alerts_enabled: self.income_alerts_enabled,
            liquidation_alerts_enabled: self.liquidation_alerts_enabled,
            liquidation_alert_threshold: self.liquidation_alert_threshold,
            market_slippage_pct: self.market_slippage_pct,
            tracked_trade_alerts_enabled: self.tracked_trade_alerts_enabled,
            tracked_trade_aggregation_enabled: self.tracked_trade_aggregation_enabled,
            liquidation_feed_aggregation_enabled: self.liquidation_feed_aggregation_enabled,
            preset_is_usd: self.preset_is_usd,
            order_presets: self.order_presets.clone(),
        }
    }
}
