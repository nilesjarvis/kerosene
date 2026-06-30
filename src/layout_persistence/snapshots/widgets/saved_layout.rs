use crate::app_state::TradingTerminal;
use crate::config;

// ---------------------------------------------------------------------------
// Saved Layout Snapshots
// ---------------------------------------------------------------------------

impl TradingTerminal {
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
        self.order_kind.config_str().to_string()
    }

    pub(crate) fn liquidation_distribution_symbol_config_value(&self) -> String {
        let symbol = self.liquidation_distribution.symbol.trim();
        if symbol.is_empty() || self.symbol_key_is_hidden(symbol) {
            String::new()
        } else {
            symbol.to_string()
        }
    }

    pub(crate) fn pane_layout_config_snapshot(&self) -> Option<config::PaneLayoutConfig> {
        let runtime_layout = self.collect_pane_layout();
        let Some(preserved_layout) = &self.preserved_loaded_pane_layout else {
            return runtime_layout;
        };

        if config::prune_unsupported_pane_layout(preserved_layout.clone()) == runtime_layout {
            Some(preserved_layout.clone())
        } else {
            runtime_layout
        }
    }

    pub(crate) fn saved_layout_snapshot(&self, name: String) -> config::SavedLayout {
        config::SavedLayout {
            name,
            pane_layout: self.pane_layout_config_snapshot(),
            layout_ratios: self.collect_layout_ratios(),
            charts: self.docked_chart_configs_snapshot(),
            order_books: self.order_book_configs_snapshot(),
            live_watchlists: self.live_watchlist_configs_snapshot(),
            positioning_infos: self.positioning_info_configs_snapshot(),
            session_data: self.session_data_configs_snapshot(),
            x_feeds: self.x_feed_configs_snapshot(),
            spaghetti_charts: self.spaghetti_chart_configs_snapshot(),
            widget_padding: self.widget_padding_config_snapshot(),
            active_symbol: self.active_symbol_config_value(),
            liquidation_distribution_symbol: Some(
                self.liquidation_distribution_symbol_config_value(),
            ),
            active_timeframe: self.active_timeframe_config_value(),
            order_kind: self.order_kind_config_value(),
            reduce_only: self.order_reduce_only,
            book_tick_size: config::default_tick_size(),
            favourite_symbols: self.favourite_symbols_config_values(),
            ticker_tape_enabled: self.ticker_tape_enabled,
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
