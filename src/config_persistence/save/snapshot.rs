use crate::app_state::TradingTerminal;
use crate::config;
use std::collections::HashMap;

impl TradingTerminal {
    fn config_snapshot_accounts(&self) -> Vec<config::AccountProfile> {
        self.accounts
            .iter()
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            .map(|profile| config::AccountProfile {
                secret_id: profile.secret_id.clone(),
                name: profile.name.clone(),
                wallet_address: profile.wallet_address.clone(),
                agent_key: String::new().into(),
                hydromancer_api_key: String::new().into(),
            })
            .collect()
    }

    /// Build a config snapshot from the current state.
    pub(super) fn config_snapshot(&self) -> config::KeroseneConfig {
        if self.config_clear_requested || self.config_cleared_this_session {
            return config::KeroseneConfig::default();
        }

        let layout_snapshot = self.saved_layout_snapshot("current".to_string());
        let persisted_accounts = self.config_snapshot_accounts();
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
            saved_layouts: self.saved_layouts_config_values(),
            active_layout_name: self.active_layout_name.clone(),
            credential_storage_mode: self.secret_storage_mode,
            encrypted_secrets: self.encrypted_secrets.clone(),
            secret_migration_save_blocked: false,
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
            chart_dotted_background: self.chart_dotted_background,
            chart_dotted_background_opacity: self.chart_dotted_background_opacity,
            chart_hollow_candles: false,
            chart_hollow_candle_mode: self.chart_hollow_candle_mode,
            chart_fisheye_enabled: self.chart_fisheye_enabled,
            chart_fisheye_strength: config::normalize_chart_fisheye_strength(
                self.chart_fisheye_strength,
            ),
            chart_chromatic_aberration_enabled: self.chart_chromatic_aberration_enabled,
            chart_chromatic_aberration_strength:
                config::normalize_chart_chromatic_aberration_strength(
                    self.chart_chromatic_aberration_strength,
                ),
            chart_edge_blur_enabled: self.chart_edge_blur_enabled,
            chart_edge_blur_strength: config::normalize_chart_edge_blur_strength(
                self.chart_edge_blur_strength,
            ),
            chart_crosshair_style: self.chart_crosshair_style.normalized(),
            chart_crosshair_guides_enabled: self.chart_crosshair_guides_enabled,
            chart_crosshair_scale: config::normalize_chart_crosshair_scale(
                self.chart_crosshair_scale,
            ),
            chart_hud_order_sound: self.chart_hud_order_sound,
            chart_hud_order_sound_file: self.chart_hud_order_sound_file.clone(),
            chart_hud_order_sound_volume: config::normalize_chart_hud_order_sound_volume(
                self.chart_hud_order_sound_volume,
            ),
            chart_hud_ui_sounds: self.chart_hud_ui_sounds,
            chart_hud_readout: self.chart_hud_readout,
            alfred_popup_scale: self.alfred_popup_scale,
            read_data_provider: self.read_data_provider,
            chart_backfill_source: self.read_data_provider.chart_backfill_source(),
            display_font: self.display_font.clone(),
            monospace_font: self.monospace_font.clone(),
            custom_fonts: self.custom_fonts.clone(),
            pane_border_thickness: self.pane_border_thickness,
            pane_corner_radius: self.pane_corner_radius,
            outer_widget_border_enabled: self.outer_widget_border_enabled,
            widget_padding: self.widget_padding_config_snapshot(),
            custom_window_chrome_enabled: self.custom_window_chrome_enabled,
            symbol_search_sort_mode: self.symbol_search_sort_mode.config_value().to_string(),
            market_universe: self.market_universe.clone().normalized(),
            liquidation_distribution_symbol: self.liquidation_distribution_symbol_config_value(),
            display_denomination: self.display_denomination.clone().normalized(),
            chart_screenshot_settings: self.chart_screenshot_settings.clone(),
            accounts: persisted_accounts,
            pending_keychain_profile_deletions: self.pending_keychain_profile_deletions.clone(),
            pending_keychain_cleanup_all: self.pending_keychain_cleanup_all,
            secret_cleanup_state_dirty: false,
            active_account_index,
            agent_key: String::new().into(),
            wallet_address: String::new(),

            main_window_width: self.main_window_size.map(|s| s.width),
            main_window_height: self.main_window_size.map(|s| s.height),
            main_window_x: self.main_window_pos.map(|p| p.x),
            main_window_y: self.main_window_pos.map(|p| p.y),
            journal_window_width: Some(self.journal.width),
            journal_window_height: Some(self.journal.height),

            live_watchlists: layout_snapshot.live_watchlists,
            positioning_infos: layout_snapshot.positioning_infos,
            session_data: layout_snapshot.session_data,

            ticker_tape_enabled: layout_snapshot.ticker_tape_enabled,
            favourite_symbols: layout_snapshot.favourite_symbols,
            muted_tickers: self.sorted_muted_tickers(),
            outcome_display_labels: self.outcome_display_labels.clone(),
            hydromancer_api_key: String::new().into(),
            hyperdash_api_key: String::new().into(),
            sound_enabled: layout_snapshot.sound_enabled,
            desktop_notifications: layout_snapshot.desktop_notifications,
            toast_position: self.toast_position,
            toast_animations_enabled: self.toast_animations_enabled,
            income_alerts_enabled: layout_snapshot.income_alerts_enabled,
            hide_pnl: self.hide_pnl,
            hidden_positions_by_account,
            liquidation_alerts_enabled: layout_snapshot.liquidation_alerts_enabled,
            liquidation_alert_threshold: layout_snapshot.liquidation_alert_threshold,
            market_slippage_pct: layout_snapshot.market_slippage_pct,
            optimistic_account_updates: self.optimistic_account_updates,
            tracked_trade_alerts_enabled: layout_snapshot.tracked_trade_alerts_enabled,
            tracked_trade_aggregation_enabled: layout_snapshot.tracked_trade_aggregation_enabled,
            liquidation_feed_aggregation_enabled: layout_snapshot
                .liquidation_feed_aggregation_enabled,
            telegram_feed_notifications_enabled: self.telegram_feed.notifications_enabled,
            telegram_feed_fast_mode_enabled: self.telegram_feed.fast_mode_enabled,
            telegram_feed_fast_api_id: self.telegram_feed.fast_api_id,
            telegram_feed_channels: self.telegram_feed.channels.clone(),
            telegram_feed_private_channels: self.telegram_feed.private_channels.clone(),
            x_feed_notifications_enabled: self.x_feed.notifications_enabled,
            x_feed_streaming_enabled: self.x_feed.streaming_enabled,
            x_feed_handles: self.x_feed.handles.clone(),
            x_bearer_token: String::new().into(),

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
