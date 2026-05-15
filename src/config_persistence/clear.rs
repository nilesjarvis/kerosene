use crate::app_state::TradingTerminal;
use crate::config::{self, AccountProfile, KeroseneConfig};
use crate::pane_management::AddWidgetPlacement;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(crate) fn config_clear_block_reason(&self) -> Option<String> {
        let chase_count = self.chase_orders.len();
        let twap_count = self.twap_orders.len();
        let open_order_count = self
            .account_data
            .as_ref()
            .map(|data| data.open_orders.len())
            .unwrap_or_default();

        if chase_count == 0
            && twap_count == 0
            && open_order_count == 0
            && self.pending_order_action.is_none()
        {
            return None;
        }

        let mut blockers = Vec::new();
        if chase_count > 0 {
            blockers.push(format!(
                "{chase_count} active chase {}",
                if chase_count == 1 { "order" } else { "orders" }
            ));
        }
        if twap_count > 0 {
            blockers.push(format!(
                "{twap_count} active TWAP {}",
                if twap_count == 1 { "order" } else { "orders" }
            ));
        }
        if open_order_count > 0 {
            blockers.push(format!(
                "{open_order_count} known open exchange {}",
                if open_order_count == 1 {
                    "order"
                } else {
                    "orders"
                }
            ));
        }
        if self.pending_order_action.is_some() {
            blockers.push("an in-flight order action".to_string());
        }

        Some(format!(
            "Clear All Configs blocked: stop/cancel and reconcile {} before deleting credentials or runtime order state.",
            blockers.join(", ")
        ))
    }

    pub(crate) fn apply_config_clear_to_runtime(&mut self, summary: config::ClearConfigSummary) {
        if let Some(message) = self.config_clear_block_reason() {
            self.secret_store_status = Some((message.clone(), true));
            self.push_toast(message, true);
            return;
        }

        let defaults = KeroseneConfig::default();

        self.config_cleared_this_session = true;
        self.saved_layouts.clear();
        self.active_layout_name = None;
        self.layout_input.clear();
        self.layout_menu_open = false;
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
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
        self.account_error = None;
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
        self.order_quantity_is_usd = defaults.order_quantity_is_usd;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::order_execution::PendingOrderAction;
    use crate::signing::ChaseOrder;
    use crate::twap_state::{TwapOrder, TwapOrderInit};
    use std::time::{Duration, Instant};
    use zeroize::Zeroizing;

    fn clear_summary() -> config::ClearConfigSummary {
        config::ClearConfigSummary {
            files_removed: 1,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        }
    }

    fn chase_order() -> ChaseOrder {
        ChaseOrder {
            id: 42,
            coin: "BTC".to_string(),
            account_address: "0xabc".to_string(),
            agent_key: Zeroizing::new("agent-key".to_string()),
            is_buy: true,
            target_size: 1.0,
            filled_size: 0.0,
            remaining_size: 1.0,
            known_oids: vec![123],
            asset: 0,
            sz_decimals: 5,
            is_spot: false,
            reduce_only: false,
            current_oid: Some(123),
            current_price: 50_000.0,
            current_price_wire: "50000".to_string(),
            initial_price: 50_000.0,
            started_at: Instant::now(),
            started_at_ms: 1_700_000_000_000,
            reprice_count: 0,
            pending_op: None,
            last_reprice_at: None,
            pending_best_price: None,
            stop_requested: false,
            stop_reason: None,
            cancel_retries: 0,
            oid_confirmed: true,
            missing_open_order_refresh_requested: false,
        }
    }

    fn twap_order() -> TwapOrder {
        let now = Instant::now();
        TwapOrder::new(TwapOrderInit {
            id: 7,
            coin: "BTC".to_string(),
            display_coin: "BTC".to_string(),
            account_address: "0xabc".to_string(),
            agent_key: Zeroizing::new("agent-key".to_string()),
            is_buy: false,
            target_size: 2.0,
            asset: 0,
            sz_decimals: 5,
            is_spot: false,
            reduce_only: false,
            min_price: 49_000.0,
            max_price: 51_000.0,
            randomize: false,
            duration: Duration::from_secs(300),
            slice_count: 5,
            now,
            started_at_ms: 1_700_000_000_000,
        })
    }

    #[test]
    fn clear_configs_is_blocked_while_chase_tracking_exists() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.wallet_key_input = "agent-key".to_string().into();
        terminal.chase_orders.insert(42, chase_order());

        let _ = terminal.update_settings(Message::ClearConfigs);

        assert!(terminal.chase_orders.contains_key(&42));
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert!(
            terminal.secret_store_status.as_ref().is_some_and(
                |(message, is_error)| *is_error && message.contains("active chase order")
            )
        );
    }

    #[test]
    fn runtime_clear_is_blocked_while_twap_tracking_exists() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.wallet_key_input = "agent-key".to_string().into();
        terminal.twap_orders.insert(7, twap_order());

        terminal.apply_config_clear_to_runtime(clear_summary());

        assert!(terminal.twap_orders.contains_key(&7));
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert!(
            terminal.secret_store_status.as_ref().is_some_and(
                |(message, is_error)| *is_error && message.contains("active TWAP order")
            )
        );
    }

    #[test]
    fn clear_configs_is_blocked_while_order_action_is_pending() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.wallet_key_input = "agent-key".to_string().into();
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _ = terminal.update_settings(Message::ClearConfigs);

        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        assert_eq!(terminal.wallet_key_input.as_str(), "agent-key");
        assert!(terminal.secret_store_status.as_ref().is_some_and(
            |(message, is_error)| *is_error && message.contains("in-flight order action")
        ));
    }
}
