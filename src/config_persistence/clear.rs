use crate::app_state::TradingTerminal;
use crate::config::{self, AccountProfile, KeroseneConfig};
use crate::pane_management::AddWidgetPlacement;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(crate) fn apply_config_clear_to_runtime(&mut self, summary: config::ClearConfigSummary) {
        let defaults = KeroseneConfig::default();

        self.config_cleared_this_session = true;
        self.saved_layouts.clear();
        self.active_layout_name = None;
        self.layout_input.clear();
        self.layout_menu_open = false;
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
        self.hotkeys.clear();
        self.chart_timeframe_hotkey_prefix = None;
        self.recording_hotkey_for = None;
        self.secret_storage_mode = config::CredentialStorageMode::default();
        self.secret_storage_selection = self.secret_storage_mode;
        self.encrypted_secrets = None;
        self.encrypted_secret_password.zeroize();
        self.encrypted_secret_confirm.zeroize();
        self.encrypted_secrets_unlocked = false;
        self.show_unlock_credentials_popup = false;
        self.config_save_due_at = None;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.add_widget_menu_open = false;
        self.add_widget_placement = AddWidgetPlacement::Below;

        let main_profile = AccountProfile {
            secret_id: config::new_secret_id(),
            name: "Main Trading".to_string(),
            wallet_address: String::new(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        };
        self.last_persisted_active_account_secret_id = Some(main_profile.secret_id.clone());
        self.accounts = vec![main_profile];
        self.active_account_index = 0;
        self.ghost_account_secret_ids.clear();
        self.wallet_key_input.zeroize();
        self.wallet_address_input.clear();
        self.connected_address = None;
        self.account_data = None;
        self.account_loading = false;
        self.account_refresh_followup_pending = false;
        self.account_error = None;
        // Mirror connect/disconnect: in-flight order decorations belong to
        // the cleared account and must not outlive it (the agent key inside a
        // pending move context in particular).
        self.pending_order_indicators.clear();
        self.pending_move_order_contexts.clear();
        self.chase_orders.clear();
        self.selected_chase_id = None;
        self.twap_orders.clear();
        self.selected_twap_id = None;
        self.twap_form = crate::twap_state::TwapOrderForm::default();
        self.last_advanced_exchange_request_at = None;
        self.pending_order_action = None;

        self.hydromancer_api_key.zeroize();
        self.hydromancer_key_input.zeroize();
        self.hyperdash_api_key.zeroize();
        self.hyperdash_key_input.zeroize();
        self.liquidations.clear();
        self.liquidation_summary_buckets.clear();
        self.liquidation_chart_buckets.clear();
        self.liquidations_status = "Disconnected".to_string();
        self.liquidations_last_rx_ms = None;
        self.liquidations_reconnect_nonce = self.liquidations_reconnect_nonce.wrapping_add(1);
        self.tracked_trades.clear();
        self.tracked_trades_status = "Disconnected".to_string();
        self.tracked_trades_last_rx_ms = None;
        self.tracked_trades_reconnect_nonce = self.tracked_trades_reconnect_nonce.wrapping_add(1);
        self.tracked_trade_seen_keys.clear();
        self.tracked_trade_seen_order.clear();

        self.wallet_tracker.add_input.clear();
        self.wallet_tracker.add_label_input.clear();
        self.wallet_tracker.tracked_addresses.clear();
        self.wallet_tracker.muted_addresses.clear();
        self.wallet_tracker.rows.clear();
        self.wallet_tracker.core_refresh_queue.clear();
        self.wallet_tracker.order_refresh_queue.clear();
        self.address_book.clear();
        self.journal.active_account_key = self.active_journal_account_key();
        self.journal.account_states.clear();
        self.journal.entries.clear();
        self.journal.clear_active_account_data();

        self.active_theme = defaults.active_theme;
        self.ui_scale = defaults.ui_scale;
        self.chart_dotted_background = defaults.chart_dotted_background;
        self.chart_dotted_background_opacity = defaults.chart_dotted_background_opacity;
        self.chart_hollow_candle_mode = defaults.chart_hollow_candle_mode;
        self.chart_fisheye_enabled = defaults.chart_fisheye_enabled;
        self.chart_fisheye_strength = defaults.chart_fisheye_strength;
        self.chart_chromatic_aberration_enabled = defaults.chart_chromatic_aberration_enabled;
        self.chart_chromatic_aberration_strength = defaults.chart_chromatic_aberration_strength;
        self.chart_edge_blur_enabled = defaults.chart_edge_blur_enabled;
        self.chart_edge_blur_strength = defaults.chart_edge_blur_strength;
        self.chart_crosshair_style = defaults.chart_crosshair_style;
        self.chart_crosshair_guides_enabled = defaults.chart_crosshair_guides_enabled;
        self.chart_crosshair_scale = defaults.chart_crosshair_scale;
        self.chart_hud_order_sound = defaults.chart_hud_order_sound;
        self.chart_hud_order_sound_file = defaults.chart_hud_order_sound_file;
        self.chart_hud_order_sound_volume = defaults.chart_hud_order_sound_volume;
        self.chart_hud_ui_sounds = defaults.chart_hud_ui_sounds;
        self.chart_hud_readout = defaults.chart_hud_readout;
        self.alfred_popup_scale = defaults.alfred_popup_scale;
        self.read_data_provider = defaults.read_data_provider;
        self.chart_backfill_source = defaults.read_data_provider.chart_backfill_source();
        let widget_padding = defaults.widget_padding.normalized();
        self.widget_padding_default = widget_padding.default_px;
        self.widget_padding_overrides = widget_padding
            .overrides
            .into_iter()
            .map(|item| (item.target, item.padding_px))
            .collect();
        self.outer_widget_border_enabled = defaults.outer_widget_border_enabled;
        self.custom_window_chrome_enabled = defaults.custom_window_chrome_enabled;
        self.custom_themes = defaults.custom_themes;
        self.sound_enabled = defaults.sound_enabled;
        self.desktop_notifications = defaults.desktop_notifications;
        self.toast_position = defaults.toast_position;
        self.toast_animations_enabled = defaults.toast_animations_enabled;
        self.income_alerts_enabled = defaults.income_alerts_enabled;
        self.hide_pnl = defaults.hide_pnl;
        self.hidden_positions_by_account.clear();
        self.show_hidden_positions = false;
        self.liquidation_alerts_enabled = defaults.liquidation_alerts_enabled;
        self.liquidation_alert_threshold = defaults.liquidation_alert_threshold;
        self.liquidation_alert_input = defaults.liquidation_alert_threshold.to_string();
        self.market_slippage_pct = defaults.market_slippage_pct;
        self.market_slippage_input = defaults.market_slippage_pct.to_string();
        self.optimistic_account_updates = defaults.optimistic_account_updates;
        self.tracked_trade_alerts_enabled = defaults.tracked_trade_alerts_enabled;
        self.tracked_trade_aggregation_enabled = defaults.tracked_trade_aggregation_enabled;
        self.liquidation_feed_aggregation_enabled = defaults.liquidation_feed_aggregation_enabled;
        self.x_feed = crate::x_feed::XFeedState::new(
            &defaults.x_feed_handles,
            defaults.x_feed_notifications_enabled,
            defaults.x_feed_streaming_enabled,
            "",
        );
        self.order_presets = defaults.order_presets;
        self.order_quantity_is_usd = defaults.order_quantity_is_usd;
        self.preset_is_usd = defaults.preset_is_usd;
        self.favourite_symbols.clear();
        self.symbol_search_result_indices.clear();
        self.symbol_search_favourite_count = 0;
        self.muted_tickers.clear();
        self.muted_ticker_input.clear();
        self.muted_ticker_status = None;
        self.apply_chart_theme_colors();
        self.sync_chart_dotted_background();
        self.sync_chart_hollow_candles();
        self.sync_chart_fisheye();
        self.sync_chart_chromatic_aberration();
        self.sync_chart_edge_blur();
        self.sync_chart_crosshair_style();
        self.sync_chart_crosshair_guides();
        self.sync_chart_crosshair_scale();
        self.sync_chart_hud_readout();

        let file_label = if summary.files_removed == 1 {
            "file"
        } else {
            "files"
        };
        let mut message = format!(
            "Configs cleared: {} config {} removed; persistence paused until restart.",
            summary.files_removed, file_label
        );
        if !summary.warnings.is_empty() {
            message.push_str(" Some OS keychain cleanup was skipped.");
        }
        self.secret_store_status = Some((message.clone(), false));
        self.push_toast(message, false);
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::TradingTerminal;
    use crate::config::ClearConfigSummary;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn clearing_configs_clears_in_flight_order_decorations() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        let pending_id = terminal.add_pending_market_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        assert!(pending_id.is_some());

        terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(!terminal.account_refresh_followup_pending);
    }
}
