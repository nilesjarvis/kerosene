use crate::app_state::TradingTerminal;
use crate::config;
use std::collections::HashMap;

impl TradingTerminal {
    /// Build a config snapshot from the current state.
    pub(super) fn config_snapshot(&self) -> config::KeroseneConfig {
        if self.config_cleared_this_session {
            return config::KeroseneConfig::default();
        }

        let layout_snapshot = self.saved_layout_snapshot("current".to_string());
        let persisted_accounts = self.persisted_accounts_snapshot();
        let active_account_index = self.persisted_active_account_index(&persisted_accounts);
        let hidden_positions_by_account =
            self.persisted_hidden_positions_by_account(&persisted_accounts);
        let journal_entries_by_account = self.persisted_journal_entries_by_account();
        let journal_entries = match self.journal.active_account_key.as_ref() {
            Some(key) if self.ghost_account_secret_ids.contains(key) => HashMap::new(),
            Some(_) => self.journal.entries.clone(),
            None => self.journal.entries.clone(),
        };

        config::KeroseneConfig {
            saved_layouts: self.saved_layouts.clone(),
            active_layout_name: self.active_layout_name.clone(),
            credential_storage_mode: self.secret_storage_mode,
            encrypted_secrets: self.encrypted_secrets.clone(),
            book_tick_size: layout_snapshot.book_tick_size,
            order_books: layout_snapshot.order_books,
            layout_ratios: layout_snapshot.layout_ratios,
            pane_layout: layout_snapshot.pane_layout,
            charts: self.chart_configs_snapshot(),
            detached_chart_windows: self.detached_chart_window_configs_snapshot(),
            active_symbol: layout_snapshot.active_symbol,
            active_timeframe: layout_snapshot.active_timeframe,
            order_kind: layout_snapshot.order_kind,
            reduce_only: layout_snapshot.reduce_only,
            order_quantity_is_usd: self.order_quantity_is_usd,
            ui_scale: self.ui_scale,
            alfred_popup_scale: self.alfred_popup_scale,
            display_font: self.display_font.clone(),
            monospace_font: self.monospace_font.clone(),
            custom_fonts: self.custom_fonts.clone(),
            pane_border_thickness: self.pane_border_thickness,
            pane_corner_radius: self.pane_corner_radius,
            symbol_search_sort_mode: self.symbol_search_sort_mode.config_value().to_string(),
            market_universe: self.market_universe.clone().normalized(),
            display_denomination: self.display_denomination.clone().normalized(),
            chart_screenshot_settings: self.chart_screenshot_settings.clone(),
            accounts: persisted_accounts,
            active_account_index,
            agent_key: String::new().into(),
            wallet_address: String::new(),

            main_window_width: self.main_window_size.map(|s| s.width),
            main_window_height: self.main_window_size.map(|s| s.height),
            main_window_x: self.main_window_pos.map(|p| p.x),
            main_window_y: self.main_window_pos.map(|p| p.y),

            live_watchlists: layout_snapshot.live_watchlists,
            positioning_infos: layout_snapshot.positioning_infos,

            ticker_tape_enabled: layout_snapshot.ticker_tape_enabled,
            favourite_symbols: layout_snapshot.favourite_symbols,
            muted_tickers: self.sorted_muted_tickers(),
            hydromancer_api_key: String::new().into(),
            hyperdash_api_key: String::new().into(),
            sound_enabled: layout_snapshot.sound_enabled,
            desktop_notifications: layout_snapshot.desktop_notifications,
            income_alerts_enabled: layout_snapshot.income_alerts_enabled,
            hide_pnl: self.hide_pnl,
            hidden_positions_by_account,
            liquidation_alerts_enabled: layout_snapshot.liquidation_alerts_enabled,
            liquidation_alert_threshold: layout_snapshot.liquidation_alert_threshold,
            market_slippage_pct: layout_snapshot.market_slippage_pct,
            tracked_trade_alerts_enabled: layout_snapshot.tracked_trade_alerts_enabled,
            tracked_trade_aggregation_enabled: layout_snapshot.tracked_trade_aggregation_enabled,
            liquidation_feed_aggregation_enabled: layout_snapshot
                .liquidation_feed_aggregation_enabled,

            spaghetti_charts: layout_snapshot.spaghetti_charts,
            wallet_tracker: self.wallet_tracker.to_config(&self.address_book),
            address_book: self.address_book_config(),
            active_theme: layout_snapshot.active_theme,
            custom_themes: layout_snapshot.custom_themes,
            journal_entries,
            journal_entries_by_account,
            order_presets: layout_snapshot.order_presets,
            advanced_order_history: self.advanced_order_history.iter().cloned().collect(),
            preset_is_usd: layout_snapshot.preset_is_usd,
            hotkeys: self.hotkeys.clone(),
            chart_timeframe_hotkey_prefix: self.chart_timeframe_hotkey_prefix,
        }
    }
}
