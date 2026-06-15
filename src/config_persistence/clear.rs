use crate::app_state::TradingTerminal;
use crate::config::{self, AccountProfile, KeroseneConfig};
use crate::message::Message;
use crate::pane_management::AddWidgetPlacement;
use crate::telegram_fast_feed::{
    clear_all_fast_channel_cursors_best_effort, clear_telegram_fast_pending_auth,
};
use crate::telegram_feed::TelegramFeedState;
use crate::ws;
use iced::Task;
use zeroize::{Zeroize, Zeroizing};

impl TradingTerminal {
    pub(crate) fn request_config_clear(&mut self) -> Task<Message> {
        if self.config_cleared_this_session {
            self.secret_store_status = Some((
                "Config persistence is already paused until restart".to_string(),
                false,
            ));
            return Task::none();
        }

        if self.config_clear_requested {
            self.secret_store_status =
                Some(("Config clear is already in progress".to_string(), false));
            return Task::none();
        }

        if self.account_change_blocked_by_pending_trading_request("clearing configs") {
            return Task::none();
        }

        if self.account_change_blocked_by_active_automation("clearing configs") {
            return Task::none();
        }

        self.config_clear_requested = true;
        self.config_save_due_at = None;
        self.config_save_exit_requested = false;

        if self.config_save_in_flight {
            let message =
                "Config clear will run after the current config save finishes".to_string();
            self.secret_store_status = Some((message.clone(), false));
            self.push_toast(message, false);
            return Task::none();
        }

        self.start_config_clear_task()
    }

    pub(crate) fn start_config_clear_task(&mut self) -> Task<Message> {
        if self.config_cleared_this_session || self.config_save_in_flight {
            return Task::none();
        }

        if self.account_change_blocked_by_pending_trading_request("clearing configs") {
            self.config_clear_requested = false;
            self.config_save_due_at = None;
            self.config_save_exit_requested = false;
            return Task::none();
        }

        if self.account_change_blocked_by_active_automation("clearing configs") {
            self.config_clear_requested = false;
            self.config_save_due_at = None;
            self.config_save_exit_requested = false;
            return Task::none();
        }

        self.config_clear_requested = true;
        self.config_save_due_at = None;
        self.config_save_exit_requested = false;

        let profiles = self.keychain_cleanup_profiles_snapshot();
        Task::perform(
            async move { config::clear_all_configs(&profiles) },
            Message::ConfigsCleared,
        )
    }

    pub(crate) fn handle_config_clear_result(
        &mut self,
        result: Result<config::ClearConfigSummary, String>,
    ) -> Task<Message> {
        match result {
            Ok(summary)
                if summary.file_cleanup_failed
                    && Self::config_clear_side_cleanup_failed_after_config_removal(&summary) =>
            {
                if self.has_pending_trading_request() {
                    return self.defer_runtime_config_clear_for_trading_activity(summary);
                }
                if self.has_active_order_automation() {
                    return self.defer_runtime_config_clear_for_trading_activity(summary);
                }
                self.apply_config_clear_to_runtime(summary)
            }
            Ok(summary) if summary.file_cleanup_failed => {
                self.config_clear_requested = false;
                let details = summary.warnings.join("; ");
                if summary.files_removed > 0 {
                    self.config_cleared_this_session = true;
                    self.config_save_due_at = None;
                    self.config_save_exit_requested = false;
                }
                let keychain_cleanup_failed = summary
                    .warnings
                    .iter()
                    .any(|warning| warning.starts_with("keychain cleanup failed"));
                let file_cleanup_failed = summary
                    .warnings
                    .iter()
                    .any(|warning| !warning.starts_with("keychain cleanup failed"));
                let failure_target = if keychain_cleanup_failed && !file_cleanup_failed {
                    "credentials could not be removed"
                } else if keychain_cleanup_failed {
                    "config cleanup could not be completed"
                } else {
                    "config files could not be removed"
                };
                let runtime_status = if summary.files_removed > 0 {
                    "runtime was not reset; persistence is paused until restart to avoid recreating removed config files"
                } else {
                    "runtime was not reset and on-disk config remains"
                };
                let message = if details.is_empty() {
                    format!("Config clear failed: {failure_target}; {runtime_status}.")
                } else {
                    format!("Config clear failed: {failure_target}; {runtime_status}. {details}")
                };
                self.secret_store_status = Some((message.clone(), true));
                self.push_toast(message, true);
                Task::none()
            }
            Ok(summary) if self.has_pending_trading_request() => {
                self.defer_runtime_config_clear_for_trading_activity(summary)
            }
            Ok(summary) if self.has_active_order_automation() => {
                self.defer_runtime_config_clear_for_trading_activity(summary)
            }
            Ok(summary) => self.apply_config_clear_to_runtime(summary),
            Err(e) => {
                self.config_clear_requested = false;
                let message = format!("Config clear failed: {e}");
                self.secret_store_status = Some((message.clone(), true));
                self.push_toast(message, true);
                Task::none()
            }
        }
    }

    fn config_clear_side_cleanup_failed_after_config_removal(
        summary: &config::ClearConfigSummary,
    ) -> bool {
        summary.files_removed > 0
            && !summary.warnings.iter().any(|warning| {
                warning.starts_with("keychain cleanup failed")
                    || warning.starts_with("config file cleanup failed")
            })
    }

    fn defer_runtime_config_clear_for_trading_activity(
        &mut self,
        summary: config::ClearConfigSummary,
    ) -> Task<Message> {
        self.config_clear_requested = false;
        self.config_cleared_this_session = true;
        self.config_save_due_at = None;
        self.config_save_exit_requested = false;

        let item_label = if summary.files_removed == 1 {
            "item"
        } else {
            "items"
        };
        let mut message = format!(
            "Configs cleared: {} config {} removed; runtime reset skipped because pending trading requests or active order automation are still running. Restart after they finish to complete reset; persistence is paused.",
            summary.files_removed, item_label
        );
        if !summary.warnings.is_empty() {
            message.push_str(" Some cleanup steps were skipped.");
        }
        self.secret_store_status = Some((message.clone(), true));
        self.push_toast(message, true);
        Task::none()
    }

    pub(crate) fn apply_config_clear_to_runtime(
        &mut self,
        summary: config::ClearConfigSummary,
    ) -> Task<Message> {
        let defaults = KeroseneConfig::default();

        self.config_clear_requested = false;
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
        self.secret_migration_save_blocked = false;
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
        self.pending_keychain_profile_deletions.clear();
        self.pending_keychain_cleanup_all = false;
        self.wallet_key_input.zeroize();
        self.wallet_address_input.clear();
        self.connected_address = None;
        self.account_data = None;
        self.account_data_address = None;
        self.account_loading = false;
        self.account_connect_pending = false;
        self.account_refresh_followup_pending = false;
        self.account_reconciliation_required = false;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
        self.account_refresh_retry_due_ms = None;
        self.clear_portfolio_income_account_state();
        self.clear_account_scoped_chart_state();
        // Mirror connect/disconnect: in-flight order decorations belong to
        // the cleared account and must not outlive it (the agent key inside a
        // pending move context in particular).
        self.pending_order_indicators.clear();
        self.pending_one_shot_status_request = None;
        self.clear_pending_move_order_state();
        self.chase_orders.clear();
        self.selected_chase_id = None;
        self.twap_orders.clear();
        self.account_twap_reconciliation_generations.clear();
        self.selected_twap_id = None;
        self.twap_form = crate::twap_state::TwapOrderForm::default();
        self.last_advanced_exchange_request_at = None;
        self.pending_order_action = None;
        let advanced_order_history_window_ids = self
            .advanced_order_history_windows
            .keys()
            .copied()
            .collect::<Vec<_>>();
        self.advanced_order_history.clear();
        self.advanced_order_history_windows.clear();

        let previous_hydromancer_key = Zeroizing::new(self.hydromancer_api_key.trim().to_string());
        let previous_hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_key_changed =
            !previous_hydromancer_key.is_empty() || !self.hydromancer_key_input.trim().is_empty();
        if !previous_hydromancer_key.is_empty() {
            ws::evict_hydromancer_manager(previous_hydromancer_generation);
        }
        self.hydromancer_api_key.zeroize();
        self.hydromancer_key_input.zeroize();
        if hydromancer_key_changed {
            self.bump_hydromancer_key_generation();
            self.journal.snapshot_requests.clear();
        }
        let hyperdash_key_changed = !self.hyperdash_api_key.trim().is_empty()
            || !self.hyperdash_key_input.trim().is_empty();
        self.hyperdash_api_key.zeroize();
        self.hyperdash_key_input.zeroize();
        if hyperdash_key_changed {
            self.bump_hyperdash_key_generation();
        }
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

        let wallet_detail_window_ids = self
            .wallet_detail_windows
            .keys()
            .copied()
            .collect::<Vec<_>>();
        self.wallet_detail_windows.clear();
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
        let read_data_provider_changed = self.read_data_provider != defaults.read_data_provider;
        self.read_data_provider = defaults.read_data_provider;
        if read_data_provider_changed {
            self.bump_read_data_provider_generation();
        }
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
        clear_telegram_fast_pending_auth();
        clear_all_fast_channel_cursors_best_effort();
        self.telegram_feed = TelegramFeedState::new(
            &defaults.telegram_feed_channels,
            &defaults.telegram_feed_private_channels,
            defaults.telegram_feed_notifications_enabled,
            defaults.telegram_feed_fast_mode_enabled,
            defaults.telegram_feed_fast_api_id,
            defaults.telegram_feed_include_outcome_markets,
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

        let item_label = if summary.files_removed == 1 {
            "item"
        } else {
            "items"
        };
        let mut message = format!(
            "Configs cleared: {} config {} removed; persistence paused until restart.",
            summary.files_removed, item_label
        );
        let has_cleanup_warnings = !summary.warnings.is_empty();
        if has_cleanup_warnings {
            message.push_str(" Some cleanup steps were skipped.");
        }
        self.secret_store_status = Some((message.clone(), has_cleanup_warnings));
        self.push_toast(message, has_cleanup_warnings);
        let backfill_task = self.reload_chart_backfills_for_source_change();
        let funding_task = self.refresh_enabled_funding_charts();
        Task::batch(
            [backfill_task, funding_task]
                .into_iter()
                .chain(
                    wallet_detail_window_ids
                        .into_iter()
                        .map(iced::window::close),
                )
                .chain(
                    advanced_order_history_window_ids
                        .into_iter()
                        .map(iced::window::close),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::account_analytics::{
        IncomeHourlyPayment, IncomeSnapshot, PortfolioBucket, PortfolioHistory,
    };
    use crate::advanced_order_history::{AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind};
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::chart::{HudSelectorKind, OrderOverlay, PositionOverlay, TradeMarker};
    use crate::chart_state::{CandleFetchRequest, ChartInstance, ChartSurfaceId};
    use crate::config::{AccountProfile, ChartBackfillSource, ClearConfigSummary};
    use crate::order_execution::{MoveOrderKey, PendingOrderAction, QuickOrderForm};
    use crate::signing::{ChaseLifecycle, ChaseOrder};
    use crate::telegram_fast_feed::{
        fast_channel_cursor_message_id_for_test, fast_channel_cursor_test_lock,
        set_fast_channel_cursor_for_test,
    };
    use crate::telegram_feed::{
        TelegramChannelProfile, TelegramFastAuthStage, TelegramFeedPost, TelegramFeedPostSource,
        TelegramFeedPrivateChannelConfig, default_telegram_feed_channels,
    };
    use crate::timeframe::Timeframe;
    use crate::twap_state::{TwapOrder, TwapOrderInit};
    use crate::wallet_state::WalletDetailsWindowState;
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn chase_order(account_address: &str) -> ChaseOrder {
        ChaseOrder {
            id: 42,
            coin: "BTC".to_string(),
            account_address: account_address.to_string(),
            agent_key: sensitive_string("old-account-agent-key")
                .into_zeroizing()
                .into(),
            is_buy: true,
            target_size: 1.0,
            filled_size: 0.0,
            remaining_size: 1.0,
            known_oids: vec![1001],
            current_cloid: None,
            place_attempt_count: 0,
            asset: 0,
            sz_decimals: 5,
            is_spot: false,
            reduce_only: false,
            current_oid: Some(1001),
            current_price: 50_000.0,
            current_price_wire: "50000".to_string(),
            initial_price: 50_000.0,
            started_at: Instant::now(),
            started_at_ms: 1,
            fill_cutoff_ms_by_oid: Vec::new(),
            reprice_count: 0,
            lifecycle: ChaseLifecycle::Resting,
            last_reprice_at: None,
            desired_price: None,
            stop_reason: None,
            cancel_retries: 0,
        }
    }

    fn twap_order(id: u64, account_address: &str) -> TwapOrder {
        let now = Instant::now();
        TwapOrder::new(TwapOrderInit {
            id,
            coin: "BTC".to_string(),
            display_coin: "BTC".to_string(),
            account_address: account_address.to_string(),
            agent_key: sensitive_string("old-account-agent-key")
                .into_zeroizing()
                .into(),
            is_buy: true,
            target_size: 1.0,
            asset: 0,
            sz_decimals: 5,
            is_spot: false,
            reduce_only: false,
            min_price: 49_000.0,
            max_price: 51_000.0,
            randomize: false,
            duration: Duration::from_secs(60),
            slice_count: 1,
            now,
            started_at_ms: TradingTerminal::now_ms(),
        })
    }

    fn portfolio_history() -> PortfolioHistory {
        let mut buckets = HashMap::new();
        buckets.insert(
            "day".to_string(),
            PortfolioBucket {
                account_value_history: vec![(1, 1.0)],
                pnl_history: vec![(1, 1.0)],
                vlm: None,
                skipped_invalid_points: 0,
                invalid_vlm: false,
            },
        );
        PortfolioHistory { buckets }
    }

    fn income_snapshot() -> IncomeSnapshot {
        IncomeSnapshot {
            earned_total: 1.0,
            earned_24h: 1.0,
            earned_7d: 1.0,
            earned_30d: 1.0,
            net_yearly_projection: 0.0,
            current_supply_usd: 0.0,
            current_borrow_usd: 0.0,
            health: "healthy".to_string(),
            health_factor: None,
            token_rows: Vec::new(),
            recent_hourly_payments: vec![IncomeHourlyPayment {
                time: 123,
                token_label: "USDC".to_string(),
                supply: 1.0,
                borrow: 0.0,
                net: 1.0,
            }],
            invalid_token_rows: 0,
            invalid_interest_rows: 0,
        }
    }

    fn quick_order_form() -> QuickOrderForm {
        QuickOrderForm {
            price: 100.0,
            quantity: "2.5".to_string(),
            quantity_is_usd: false,
            percentage: 25.0,
            quantity_provenance: None,
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        }
    }

    fn seed_account_scoped_chart_state(terminal: &mut TradingTerminal) {
        terminal.charts.clear();
        terminal.chart_quick_order_surface.clear();

        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.set_quick_order(quick_order_form());
        instance.chart.set_hud_armed_at(true, 1_000);
        instance.chart.active_position = Some(PositionOverlay {
            entry_px: 50_000.0,
            szi: 1.0,
            liquidation_px: Some(40_000.0),
        });
        instance.chart.active_orders.push(OrderOverlay {
            coin: "BTC".to_string(),
            limit_px: 51_000.0,
            sz: 1.0,
            is_buy: true,
            oid: 1001,
            is_moving: false,
            pending_state: None,
        });
        instance.chart.trade_markers.push(TradeMarker {
            time_ms: 123,
            price: 50_500.0,
            size: 1.0,
            is_buy: true,
        });
        instance.chart.start_hud_order_animation(
            50_000.0,
            iced::Point::new(10.0, 20.0),
            iced::Size::new(300.0, 200.0),
            true,
            true,
        );
        instance
            .chart
            .set_pending_market_order_loading([(77, true)]);
        instance
            .chart
            .push_hud_feed("MKT LONG 1 @ 50000".to_string(), true, 1_000);
        instance
            .chart
            .open_hud_weapon_selector(HudSelectorKind::Mode, true);
        instance.chart.hud_max_notional = Some(1_000.0);
        assert!(instance.chart.hud_animation_tick_needed());

        terminal.charts.insert(7, instance);
        terminal
            .chart_quick_order_surface
            .insert(7, ChartSurfaceId::Docked(7));
    }

    fn assert_account_scoped_chart_state_cleared(terminal: &TradingTerminal) {
        let instance = terminal.charts.get(&7).expect("seeded chart");
        assert!(instance.quick_order.is_none());
        assert!(!instance.chart.quick_order_open);
        assert_eq!(instance.chart.quick_order_limit_price, None);
        assert_eq!(instance.last_quick_order_quantity, "");
        assert!(!instance.chart.hud_armed());
        assert!(instance.chart.active_position.is_none());
        assert!(instance.chart.active_orders.is_empty());
        assert!(instance.chart.trade_markers.is_empty());
        assert!(instance.chart.hud_order_animation.is_none());
        assert!(instance.chart.hud_feed.is_empty());
        assert!(instance.chart.hud_weapon_selector.is_none());
        assert_eq!(instance.chart.hud_max_notional, None);
        assert!(!instance.chart.hud_animation_tick_needed());
        assert!(terminal.chart_quick_order_surface.is_empty());
    }

    #[test]
    fn clear_request_blocks_while_trading_request_is_pending() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.request_config_clear();

        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        let toast = terminal.toasts.last().expect("blocked clear should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before clearing configs")
        );
    }

    #[test]
    fn clear_request_blocks_while_chase_order_is_active() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.chase_orders.insert(42, chase_order(TEST_ACCOUNT));

        let _task = terminal.request_config_clear();

        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.chase_orders.contains_key(&42));
        let toast = terminal.toasts.last().expect("blocked clear should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains(
            "Stop active chase orders and wait for cancellation to finish before clearing configs"
        ));
    }

    #[test]
    fn clear_request_blocks_while_twap_order_is_active() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.twap_orders.insert(7, twap_order(7, TEST_ACCOUNT));

        let _task = terminal.request_config_clear();

        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.twap_orders.contains_key(&7));
        let toast = terminal.toasts.last().expect("blocked clear should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains(
            "Stop active TWAP orders and wait for cancellation to finish before clearing configs"
        ));
    }

    #[test]
    fn deferred_clear_rechecks_pending_trading_before_starting() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.config_save_in_flight = true;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.handle_config_save_result(Ok(()));

        assert!(!terminal.config_save_in_flight);
        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        let toast = terminal
            .toasts
            .last()
            .expect("blocked deferred clear should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before clearing configs")
        );
    }

    #[test]
    fn deferred_clear_rechecks_active_automation_before_starting() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.config_save_in_flight = true;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.twap_orders.insert(7, twap_order(7, TEST_ACCOUNT));

        let _task = terminal.handle_config_save_result(Ok(()));

        assert!(!terminal.config_save_in_flight);
        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.twap_orders.contains_key(&7));
        let toast = terminal
            .toasts
            .last()
            .expect("blocked deferred clear should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains(
            "Stop active TWAP orders and wait for cancellation to finish before clearing configs"
        ));
    }

    #[test]
    fn clear_result_preserves_runtime_when_trading_request_becomes_pending() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.pending_order_action = Some(PendingOrderAction::Sell);

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 1,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        }));

        assert!(!terminal.config_clear_requested);
        assert!(terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::Sell)
        );
        let status = terminal
            .secret_store_status
            .as_ref()
            .expect("clear result should report deferred reset");
        assert!(status.1);
        assert!(status.0.contains("runtime reset skipped"));
        assert!(
            status
                .0
                .contains("Restart after they finish to complete reset")
        );
    }

    #[test]
    fn clear_result_preserves_runtime_when_active_chase_appears_after_clear_started() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.chase_orders.insert(42, chase_order(TEST_ACCOUNT));

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 1,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        }));

        assert!(!terminal.config_clear_requested);
        assert!(terminal.config_cleared_this_session);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert!(terminal.chase_orders.contains_key(&42));
        let status = terminal
            .secret_store_status
            .as_ref()
            .expect("clear result should report deferred reset");
        assert!(status.1);
        assert!(status.0.contains("runtime reset skipped"));
        assert!(
            status
                .0
                .contains("active order automation are still running")
        );
        assert!(
            status
                .0
                .contains("Restart after they finish to complete reset")
        );
    }

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
        terminal.account_refresh_followup_pending = true;
        terminal.account_reconciliation_required = true;
        terminal.account_refresh_backoff_until_ms = Some(TradingTerminal::now_ms() + 60_000);
        terminal.account_refresh_retry_due_ms = terminal.account_refresh_backoff_until_ms;
        terminal.active_move_order_drag = Some(MoveOrderKey::new("BTC", 1001));
        let portfolio_request_id = terminal.portfolio.begin_refresh();
        terminal.portfolio.queue_refresh_followup();
        terminal.portfolio.data = Some(portfolio_history());
        terminal.portfolio.last_error = Some("old portfolio error".to_string());
        let income_request_id = terminal.income.begin_refresh();
        terminal.income.queue_refresh_followup();
        terminal.income.data = Some(income_snapshot());
        terminal.income.last_error = Some("old income error".to_string());
        terminal.last_income_alert_time = Some(123);
        seed_account_scoped_chart_state(&mut terminal);

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(terminal.active_move_order_drag.is_none());
        assert!(!terminal.account_refresh_followup_pending);
        assert!(!terminal.account_reconciliation_required);
        assert!(terminal.account_refresh_backoff_until_ms.is_none());
        assert!(terminal.account_refresh_retry_due_ms.is_none());
        assert_ne!(terminal.portfolio.refresh_request_id, portfolio_request_id);
        assert!(!terminal.portfolio.loading);
        assert!(!terminal.portfolio.refresh_followup_pending);
        assert!(terminal.portfolio.data.is_none());
        assert!(terminal.portfolio.last_error.is_none());
        assert_ne!(terminal.income.refresh_request_id, income_request_id);
        assert!(!terminal.income.loading);
        assert!(!terminal.income.refresh_followup_pending);
        assert!(terminal.income.data.is_none());
        assert!(terminal.income.last_error.is_none());
        assert!(terminal.last_income_alert_time.is_none());
        assert_account_scoped_chart_state_cleared(&terminal);
    }

    #[test]
    fn clearing_configs_clears_pending_keychain_profile_deletions_after_runtime_reset() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal
            .pending_keychain_profile_deletions
            .push("acct-deleted".to_string());

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 1,
            file_cleanup_failed: false,
            keychain_entries_cleared: 8,
            warnings: Vec::new(),
        });

        assert!(terminal.pending_keychain_profile_deletions.is_empty());
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("success status");
        assert!(!*is_error);
        assert!(!status.contains("acct-deleted"));
    }

    #[test]
    fn clearing_configs_clears_wallet_detail_advanced_history_and_twap_runtime_state() {
        let (mut terminal, _) = TradingTerminal::boot();
        let wallet_detail_window_id = iced::window::Id::unique();
        terminal.wallet_detail_windows.insert(
            wallet_detail_window_id,
            WalletDetailsWindowState::new(TEST_ACCOUNT.to_string()),
        );
        let advanced_history_window_id = iced::window::Id::unique();
        terminal
            .advanced_order_history
            .push_back(AdvancedOrderHistoryEntry {
                id: "twap-history".to_string(),
                kind: AdvancedOrderHistoryKind::Twap,
                source_id: 7,
                account_address: TEST_ACCOUNT.to_string(),
                coin: "BTC".to_string(),
                display_coin: "BTC".to_string(),
                is_buy: true,
                target_size: 1.0,
                filled_size: 1.0,
                remaining_size: 0.0,
                average_price: Some(50_000.0),
                last_working_price: Some(50_000.0),
                gross_notional: 50_000.0,
                total_fee: 1.0,
                closed_pnl: 0.0,
                min_price: None,
                max_price: None,
                reduce_only: false,
                randomize: false,
                slice_count: 1,
                slices_sent: 1,
                reprice_count: 0,
                status: "Completed".to_string(),
                summary: "Completed".to_string(),
                started_at_ms: 1,
                completed_at_ms: 2,
                logs: Vec::new(),
                children: Vec::new(),
            });
        terminal
            .advanced_order_history_windows
            .insert(advanced_history_window_id, "twap-history".to_string());
        terminal
            .account_twap_reconciliation_generations
            .insert(TEST_ACCOUNT.to_string(), 7);

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });

        assert!(terminal.wallet_detail_windows.is_empty());
        assert!(terminal.advanced_order_history.is_empty());
        assert!(terminal.advanced_order_history_windows.is_empty());
        assert!(terminal.account_twap_reconciliation_generations.is_empty());
    }

    #[test]
    fn clearing_configs_replaces_stale_hydromancer_chart_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.chart_backfill_source = ChartBackfillSource::Hydromancer;
        terminal.hydromancer_api_key = sensitive_string("old-hydro");
        terminal.hydromancer_key_input = sensitive_string("old-hydro");
        terminal.charts.clear();

        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.candle_fetch_request = Some(CandleFetchRequest {
            chart_id: 7,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        });
        terminal.charts.insert(7, instance);

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });

        let request = terminal.charts[&7]
            .candle_fetch_request
            .as_ref()
            .expect("fresh candle request");
        assert_eq!(request.source, terminal.chart_backfill_source);
        assert_eq!(
            request.hydromancer_key_generation,
            terminal.hydromancer_key_generation
        );
    }

    #[tokio::test]
    async fn clearing_configs_resets_telegram_runtime_state() {
        let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
        let (mut terminal, _) = TradingTerminal::boot();
        let cursor_channel = format!("marketfeed-clear-test-{}", TradingTerminal::now_ms());
        set_fast_channel_cursor_for_test(&cursor_channel, 99).await;
        terminal.telegram_feed.channels = vec!["privatealpha".to_string()];
        terminal.telegram_feed.private_channels = vec![TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Alpha".to_string(),
        }];
        terminal.telegram_feed.notifications_enabled = true;
        terminal.telegram_feed.fast_mode_enabled = true;
        terminal.telegram_feed.fast_api_id = Some(12345);
        terminal.telegram_feed.fast_api_id_input = "12345".to_string();
        terminal.telegram_feed.fast_api_hash_input = sensitive_string("telegram-api-hash");
        terminal.telegram_feed.fast_phone_input = "+15555555555".to_string();
        terminal.telegram_feed.fast_code_input = sensitive_string("12345");
        terminal.telegram_feed.fast_password_input = sensitive_string("telegram-password");
        terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
        terminal.telegram_feed.fast_auth_in_flight = true;
        terminal.telegram_feed.fast_connected = true;
        terminal.telegram_feed.fast_status =
            Some(("Fast Telegram mode listening".to_string(), false));
        terminal.telegram_feed.fast_password_hint = Some("secret hint".to_string());
        terminal.telegram_feed.fast_reconnect_nonce = 99;
        terminal.telegram_feed.fast_last_event_ms = Some(123);
        terminal
            .telegram_feed
            .channel_profiles
            .insert("private:42".to_string(), telegram_profile());
        terminal.telegram_feed.posts.push(telegram_post());
        terminal.telegram_feed.loading_channels = vec!["privatealpha".to_string()];
        terminal.telegram_feed.background_loading_channels = vec!["privatealpha".to_string()];
        terminal.telegram_feed.last_error = Some("old telegram error".to_string());
        terminal.telegram_feed.last_refresh_ms = Some(456);

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });

        assert_eq!(
            terminal.telegram_feed.channels,
            default_telegram_feed_channels()
        );
        assert!(terminal.telegram_feed.private_channels.is_empty());
        assert!(terminal.telegram_feed.private_channel_candidates.is_empty());
        assert!(!terminal.telegram_feed.notifications_enabled);
        assert!(!terminal.telegram_feed.fast_mode_enabled);
        assert_eq!(terminal.telegram_feed.fast_api_id, None);
        assert!(terminal.telegram_feed.fast_api_id_input.is_empty());
        assert!(terminal.telegram_feed.fast_api_hash_input.is_empty());
        assert!(terminal.telegram_feed.fast_phone_input.is_empty());
        assert!(terminal.telegram_feed.fast_code_input.is_empty());
        assert!(terminal.telegram_feed.fast_password_input.is_empty());
        assert_eq!(
            terminal.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::Idle
        );
        assert!(!terminal.telegram_feed.fast_auth_in_flight);
        assert!(!terminal.telegram_feed.fast_connected);
        assert_eq!(terminal.telegram_feed.fast_status, None);
        assert_eq!(terminal.telegram_feed.fast_password_hint, None);
        assert_eq!(terminal.telegram_feed.fast_reconnect_nonce, 0);
        assert_eq!(terminal.telegram_feed.fast_last_event_ms, None);
        assert!(terminal.telegram_feed.channel_profiles.is_empty());
        assert!(terminal.telegram_feed.posts.is_empty());
        assert!(terminal.telegram_feed.loading_channels.is_empty());
        assert!(
            terminal
                .telegram_feed
                .background_loading_channels
                .is_empty()
        );
        assert_eq!(terminal.telegram_feed.last_error, None);
        assert_eq!(terminal.telegram_feed.last_refresh_ms, None);
        assert_eq!(
            fast_channel_cursor_message_id_for_test(&cursor_channel).await,
            0
        );
    }

    #[test]
    fn clear_request_waits_for_in_flight_save_then_starts_clear() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_save_in_flight = true;
        terminal.config_save_due_at = Some(Instant::now());

        let _task = terminal.request_config_clear();

        assert!(terminal.config_clear_requested);
        assert!(terminal.config_save_in_flight);
        assert!(terminal.config_save_due_at.is_none());

        let _task = terminal.handle_config_save_result(Err("disk full".to_string()));

        assert!(terminal.config_clear_requested);
        assert!(!terminal.config_save_in_flight);
        assert!(terminal.config_save_due_at.is_none());
        assert!(
            !terminal
                .secret_store_status
                .as_ref()
                .is_some_and(|(status, _)| status.contains("Config save failed")),
            "clear requests should ignore stale save completion status"
        );
    }

    #[test]
    fn config_file_cleanup_failure_does_not_reset_runtime() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Keep Me".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        }];

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: true,
            keychain_entries_cleared: 0,
            warnings: vec!["config file cleanup failed: permission denied".to_string()],
        }));

        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts[0].name, "Keep Me");
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("clear failure status");
        assert!(*is_error);
        assert!(status.contains("runtime was not reset"));
        assert!(status.contains("on-disk config remains"));
    }

    #[test]
    fn sensitive_side_file_cleanup_failure_after_config_removal_resets_runtime() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.wallet_key_input = sensitive_string("agent-key");
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.hydromancer_api_key = sensitive_string("hydro-key");
        terminal.hydromancer_key_input = sensitive_string("hydro-key");
        terminal.hyperdash_api_key = sensitive_string("hyperdash-key");
        terminal.hyperdash_key_input = sensitive_string("hyperdash-key");
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Keep Me".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: sensitive_string("agent-key").into_zeroizing(),
            hydromancer_api_key: sensitive_string("legacy-hydro").into_zeroizing(),
        }];

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 1,
            file_cleanup_failed: true,
            keychain_entries_cleared: 0,
            warnings: vec!["Telegram session cleanup failed: permission denied".to_string()],
        }));

        assert!(!terminal.config_clear_requested);
        assert!(terminal.config_cleared_this_session);
        assert!(terminal.config_save_due_at.is_none());
        assert!(terminal.wallet_address_input.is_empty());
        assert!(terminal.wallet_key_input.is_empty());
        assert!(terminal.connected_address.is_none());
        assert!(terminal.hydromancer_api_key.is_empty());
        assert!(terminal.hydromancer_key_input.is_empty());
        assert!(terminal.hyperdash_api_key.is_empty());
        assert!(terminal.hyperdash_key_input.is_empty());
        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.accounts[0].name, "Main Trading");
        assert!(terminal.accounts[0].agent_key.is_empty());
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("clear failure status");
        assert!(*is_error);
        assert!(status.contains("Configs cleared"));
        assert!(status.contains("persistence paused until restart"));
        assert!(status.contains("Some cleanup steps were skipped"));
        assert!(!status.contains("runtime was not reset"));
        assert!(!status.contains("permission denied"));
    }

    #[test]
    fn keychain_cleanup_failure_without_config_removal_keeps_config_and_runtime() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal
            .pending_keychain_profile_deletions
            .push("acct-deleted".to_string());
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Reset Me".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        }];

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: true,
            keychain_entries_cleared: 0,
            warnings: vec!["keychain cleanup failed: keychain locked".to_string()],
        }));

        assert!(!terminal.config_clear_requested);
        assert!(!terminal.config_cleared_this_session);
        assert_eq!(
            terminal.pending_keychain_profile_deletions.as_slice(),
            ["acct-deleted".to_string()]
        );
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.accounts[0].name, "Reset Me");
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("warning status");
        assert!(*is_error);
        assert!(status.contains("credentials could not be removed"));
        assert!(status.contains("runtime was not reset"));
        assert!(status.contains("on-disk config remains"));
        assert!(!status.contains("persistence is paused until restart"));
        assert!(status.contains("keychain cleanup failed"));
        assert!(!status.contains("acct-deleted"));
    }

    #[test]
    fn keychain_cleanup_failure_after_config_removal_pauses_persistence_without_runtime_reset() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.config_save_due_at = Some(Instant::now());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal
            .pending_keychain_profile_deletions
            .push("acct-deleted".to_string());
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Reset Me".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        }];

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 2,
            file_cleanup_failed: true,
            keychain_entries_cleared: 0,
            warnings: vec!["keychain cleanup failed: keychain locked".to_string()],
        }));

        assert!(!terminal.config_clear_requested);
        assert!(terminal.config_cleared_this_session);
        assert!(terminal.config_save_due_at.is_none());
        assert_eq!(
            terminal.pending_keychain_profile_deletions.as_slice(),
            ["acct-deleted".to_string()]
        );
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.accounts[0].name, "Reset Me");
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("warning status");
        assert!(*is_error);
        assert!(status.contains("credentials could not be removed"));
        assert!(status.contains("runtime was not reset"));
        assert!(status.contains("persistence is paused until restart"));
        assert!(!status.contains("on-disk config remains"));
        assert!(status.contains("keychain cleanup failed"));
        assert!(!status.contains("acct-deleted"));
    }

    #[test]
    fn ancillary_config_cleanup_warning_still_resets_runtime() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.config_clear_requested = true;
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Reset Me".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        }];

        let _task = terminal.handle_config_clear_result(Ok(ClearConfigSummary {
            files_removed: 2,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: vec![
                "asset cleanup failed: remove fonts failed".to_string(),
                "keychain cleanup failed for acct-a Reset Me".to_string(),
            ],
        }));

        assert!(!terminal.config_clear_requested);
        assert!(terminal.config_cleared_this_session);
        assert!(terminal.wallet_address_input.is_empty());
        assert_eq!(terminal.accounts.len(), 1);
        assert_eq!(terminal.accounts[0].name, "Main Trading");
        assert_ne!(terminal.accounts[0].secret_id, "acct-a");
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("warning status");
        assert!(*is_error);
        assert!(status.contains("Configs cleared"));
        assert!(status.contains("Some cleanup steps were skipped"));
        assert!(!status.contains("remove fonts failed"));
        assert!(!status.contains("acct-a"));
        assert!(!status.contains("Reset Me"));
    }

    fn telegram_profile() -> TelegramChannelProfile {
        TelegramChannelProfile {
            channel: "private:42".to_string(),
            title: "Private Alpha".to_string(),
            initials: "PA".to_string(),
            avatar_url: None,
            avatar_handle: None,
            avatar_loading_url: None,
            avatar_request_id: 0,
            avatar_failed_at_ms: None,
        }
    }

    fn telegram_post() -> TelegramFeedPost {
        TelegramFeedPost {
            channel: "private:42".to_string(),
            message_id: 1,
            text: "BTC update".to_string(),
            timestamp_ms: 1,
            source: TelegramFeedPostSource::FastLive,
            received_at_ms: 1,
            applied_at_ms: 1,
            fetched_at_ms: 1,
            request_started_ms: 1,
            request_duration_ms: 1,
            first_seen_ms: 1,
            url: "https://t.me/c/42/1".to_string(),
            ticker_mentions: Vec::new(),
        }
    }
}
