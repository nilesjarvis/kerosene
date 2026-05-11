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
        self.hotkeys.clear();
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
        self.account_error = None;
        self.active_chase = None;
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
        self.wallet_tracker.rows.clear();
        self.wallet_tracker.core_refresh_queue.clear();
        self.wallet_tracker.order_refresh_queue.clear();
        self.address_book.clear();
        self.journal.active_account_key = self.active_journal_account_key();
        self.journal.account_states.clear();
        self.journal.entries.clear();
        self.journal.clear_active_account_data();

        self.active_theme = defaults.active_theme;
        self.custom_themes = defaults.custom_themes;
        self.sound_enabled = defaults.sound_enabled;
        self.desktop_notifications = defaults.desktop_notifications;
        self.income_alerts_enabled = defaults.income_alerts_enabled;
        self.hide_pnl = defaults.hide_pnl;
        self.hidden_positions_by_account.clear();
        self.show_hidden_positions = false;
        self.liquidation_alerts_enabled = defaults.liquidation_alerts_enabled;
        self.liquidation_alert_threshold = defaults.liquidation_alert_threshold;
        self.liquidation_alert_input = defaults.liquidation_alert_threshold.to_string();
        self.market_slippage_pct = defaults.market_slippage_pct;
        self.market_slippage_input = defaults.market_slippage_pct.to_string();
        self.tracked_trade_alerts_enabled = defaults.tracked_trade_alerts_enabled;
        self.tracked_trade_aggregation_enabled = defaults.tracked_trade_aggregation_enabled;
        self.liquidation_feed_aggregation_enabled = defaults.liquidation_feed_aggregation_enabled;
        self.order_presets = defaults.order_presets;
        self.preset_is_usd = defaults.preset_is_usd;
        self.favourite_symbols.clear();
        self.symbol_search_result_indices.clear();
        self.symbol_search_favourite_count = 0;
        self.muted_tickers.clear();
        self.muted_ticker_input.clear();
        self.muted_ticker_status = None;
        self.apply_chart_theme_colors();

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
