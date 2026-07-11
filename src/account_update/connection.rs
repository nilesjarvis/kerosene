mod refresh;

use crate::account::fetch_account_data_scoped_with_provider;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn connect_wallet(&mut self) -> Task<Message> {
        #[cfg(not(test))]
        {
            self.connect_wallet_with_hooks(
                config::save_config,
                |terminal, accounts, removed_profile_secret_id| {
                    terminal.persist_profile_agent_key_removal_from_accounts(
                        accounts,
                        removed_profile_secret_id,
                    )
                },
            )
        }
        #[cfg(test)]
        {
            self.connect_wallet_with_hooks(
                |_| Ok(()),
                |terminal, accounts, removed_profile_secret_id| {
                    terminal.persist_profile_agent_key_removal_from_accounts(
                        accounts,
                        removed_profile_secret_id,
                    )
                },
            )
        }
    }

    fn connect_wallet_with_hooks(
        &mut self,
        mut save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        mut persist_profile_agent_key_removal: impl FnMut(
            &mut Self,
            &[config::AccountProfile],
            &str,
        ) -> bool,
    ) -> Task<Message> {
        // The deferred connect (account switch / boot) is now being processed,
        // so the connecting-skeleton bridge is no longer needed; every path
        // below settles `connected_address` to its real value.
        self.account_connect_pending = false;
        let Some(addr) = Self::normalize_wallet_address(&self.wallet_address_input) else {
            if !self.wallet_address_input.trim().is_empty() {
                if self.account_change_blocked_by_pending_trading_request("changing wallets") {
                    return Task::none();
                }
                if self.account_change_blocked_by_active_automation("changing wallets") {
                    return Task::none();
                }
                self.invalidate_account_data_requests();
                self.clear_percentage_order_quantity();
                self.bump_account_data_revision();
                self.rotate_account_user_data_stream();
                if let Some(previous_address) = self.connected_address.clone() {
                    self.rotate_wallet_detail_user_data_stream_if_open(&previous_address);
                }
                self.connected_address = None;
                self.account_data = None;
                self.account_data_address = None;
                self.pending_order_indicators.clear();
                self.hud_placements.clear();
                self.pending_one_shot_status_requests.clear();
                self.pending_cancel_status_request = None;
                self.pending_move_status_request = None;
                self.clear_pending_move_order_state();
                self.pending_leverage_update = None;
                self.order_leverage_dropdown_open = false;
                self.account_loading = false;
                self.account_refresh_followup_pending = false;
                self.account_reconciliation_required = false;
                self.account_error = Some("Invalid wallet address".to_string());
                self.account_refresh_backoff_until_ms = None;
                self.account_refresh_retry_due_ms = None;
                self.clear_portfolio_income_account_state();
                self.clear_account_scoped_chart_state();
                if self.journal.window_id.is_some() {
                    self.journal.clear_active_account_data();
                    self.journal.error = Some("Invalid wallet address".to_string());
                }
                self.sync_all_chart_overlays();
                self.push_toast("Invalid wallet address".to_string(), true);
            }
            return Task::none();
        };

        self.wallet_address_input = addr.clone();
        if self.account_change_blocked_by_pending_trading_request("connecting a wallet") {
            return Task::none();
        }

        let previous_connected_address = self
            .connected_address
            .as_deref()
            .and_then(Self::normalize_wallet_address);
        let changing_account_context = previous_connected_address
            .as_deref()
            .is_none_or(|current| current != addr);
        if changing_account_context
            && self.account_change_blocked_by_active_chase("connecting another wallet")
        {
            return Task::none();
        }
        if changing_account_context
            && self.account_change_blocked_by_uncertain_twap("connecting another wallet")
        {
            return Task::none();
        }

        let mut rebinding_config_already_persisted = false;
        let mut profile_address_binding_changed = false;
        let credentials_persisted = if self.active_account_is_ghost() {
            self.wallet_key_input.zeroize();
            if let Some(profile) = self.accounts.get_mut(self.active_account_index) {
                profile.agent_key.zeroize();
            }
            true
        } else if self.active_account_index < self.accounts.len() {
            let mut previous_wallet_address = self.accounts[self.active_account_index]
                .wallet_address
                .clone();
            let previous_normalized = Self::normalize_wallet_address(&previous_wallet_address);
            let address_binding_changed = previous_normalized.as_deref() != Some(addr.as_str());

            if address_binding_changed {
                let had_agent_key = !self.wallet_key_input.trim().is_empty()
                    || !self.accounts[self.active_account_index]
                        .agent_key
                        .trim()
                        .is_empty();
                let mut removed_profile_secret_id =
                    self.accounts[self.active_account_index].secret_id.clone();
                let rollback = self
                    .begin_active_profile_address_rebind(self.active_account_index, addr.clone())
                    .expect("active profile must exist after the connect bounds check");

                let mut os_metadata_saved = false;
                let persisted = {
                    let persisted_accounts = self.persisted_accounts_snapshot();
                    match self.secret_storage_mode {
                        config::CredentialStorageMode::OsKeychain => {
                            match self.persist_config_immediately_with(&mut save_config) {
                                Ok(()) => {
                                    os_metadata_saved = true;
                                    persist_profile_agent_key_removal(
                                        self,
                                        &persisted_accounts,
                                        &removed_profile_secret_id,
                                    )
                                }
                                Err(error) => {
                                    let error = redact_sensitive_response_text(&error);
                                    self.secret_store_status = Some((
                                        format!(
                                            "Config save failed: {error}. Wallet was not connected; retry after config persistence is available."
                                        ),
                                        true,
                                    ));
                                    false
                                }
                            }
                        }
                        config::CredentialStorageMode::EncryptedConfig => {
                            persist_profile_agent_key_removal(
                                self,
                                &persisted_accounts,
                                &removed_profile_secret_id,
                            )
                        }
                    }
                };
                removed_profile_secret_id.zeroize();
                if !persisted {
                    let detail = self
                        .secret_store_status
                        .as_ref()
                        .map(|(message, _)| message.clone())
                        .unwrap_or_else(|| "Credential storage update failed".to_string());
                    let detail = redact_sensitive_response_text(&detail);
                    rollback.restore(self, previous_wallet_address);
                    if self.secret_storage_mode == config::CredentialStorageMode::OsKeychain
                        && os_metadata_saved
                        && let Err(error) = self
                            .persist_config_immediately_for_secret_migration_rollback_with(
                                &mut save_config,
                            )
                    {
                        let error = redact_sensitive_response_text(&error);
                        self.secret_store_status = Some((
                            format!(
                                "{detail}. Wallet was not connected, but saving the rollback failed: {error}. Retry after config persistence is available."
                            ),
                            true,
                        ));
                        return Task::none();
                    }
                    self.secret_store_status = Some((
                        format!(
                            "{detail}. Wallet was not connected; retry after credential storage is available."
                        ),
                        true,
                    ));
                    return Task::none();
                }
                rollback.scrub_after_commit();
                previous_wallet_address.zeroize();

                self.secret_store_status = Some((
                    if had_agent_key {
                        "Agent key cleared for this account and removed from credential storage; re-enter and save credentials for the new wallet address to trade."
                    } else {
                        "Saved agent key binding removed from credential storage for this account; save credentials for the new wallet address before trading."
                    }
                    .to_string(),
                    true,
                ));
                rebinding_config_already_persisted = true;
                profile_address_binding_changed = true;
                true
            } else {
                self.accounts[self.active_account_index].wallet_address = addr.clone();
                self.persist_active_profile_secrets()
            }
        } else {
            true
        };
        let stop_chase_ids: Vec<u64> = self
            .chase_orders
            .iter()
            .filter_map(|(id, chase)| {
                (!chase.lifecycle.is_stopping() && chase.account_address.as_str() != addr.as_str())
                    .then_some(*id)
            })
            .collect();
        let stop_chase_task = Task::batch(stop_chase_ids.into_iter().map(|id| {
            self.stop_chase_by_id_with_reason(id, "Chase stopped: wallet address changed", false)
        }));
        let stop_twap_ids: Vec<u64> = self
            .twap_orders
            .iter()
            .filter_map(|(id, twap)| {
                (!twap.status.is_terminal()
                    && !twap.stop_requested
                    && twap.account_address.as_str() != addr.as_str())
                .then_some(*id)
            })
            .collect();
        for id in stop_twap_ids {
            let _ = self.stop_twap_with_reason(id, "TWAP stopped: wallet address changed", false);
        }
        self.rotate_account_user_data_stream();
        if changing_account_context {
            if let Some(previous_address) = previous_connected_address {
                self.rotate_wallet_detail_user_data_stream_if_open(&previous_address);
            }
            self.rotate_wallet_detail_user_data_stream_if_open(&addr);
        }
        let selected_cluster_profile_binding_changed = profile_address_binding_changed
            && self
                .accounts
                .get(self.active_account_index)
                .is_some_and(|profile| {
                    self.selected_wallet_cluster_uses_profile(&profile.secret_id)
                });
        if selected_cluster_profile_binding_changed {
            self.rotate_wallet_cluster_user_data_streams();
        }
        self.connected_address = Some(addr.clone());
        self.clear_percentage_order_quantity();
        self.bump_account_data_revision();
        self.account_data = None;
        self.account_data_address = None;
        self.pending_order_indicators.clear();
        self.hud_placements.clear();
        self.pending_one_shot_status_requests.clear();
        self.pending_cancel_status_request = None;
        self.pending_move_status_request = None;
        self.clear_pending_move_order_state();
        self.pending_leverage_update = None;
        self.order_leverage_dropdown_open = false;
        self.account_loading = true;
        self.account_refresh_followup_pending = false;
        self.account_reconciliation_required = true;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
        self.account_refresh_retry_due_ms = None;
        self.clear_portfolio_income_account_state();
        if changing_account_context {
            self.clear_account_scoped_chart_state();
        } else {
            for instance in self.charts.values_mut() {
                instance.chart.clear_hud_armed();
            }
        }
        self.sync_all_chart_overlays();
        if credentials_persisted && !rebinding_config_already_persisted {
            self.persist_config();
        }

        let account_addr = addr.clone();
        let account_scope = self.account_data_fetch_scope();
        let account_provider = self.read_data_provider;
        let account_context = self.begin_account_data_request_context();
        let hydromancer_key = self.hydromancer_api_key_for_task();
        let account_task = Task::perform(
            fetch_account_data_scoped_with_provider(
                addr.clone(),
                account_scope,
                account_provider,
                hydromancer_key,
            ),
            move |r| {
                Message::AccountDataLoaded(
                    account_addr.clone().into(),
                    account_context,
                    Box::new(r),
                )
            },
        );
        let mut tasks = vec![account_task];
        tasks.push(stop_chase_task);
        tasks.push(self.start_portfolio_refresh_for_address(addr));
        tasks.extend(self.mids_bootstrap_tasks());
        tasks.push(self.load_journal_for_active_account(false));
        Task::batch(tasks)
    }

    pub(super) fn disconnect_wallet(&mut self) -> Task<Message> {
        self.account_connect_pending = false;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        if self.account_change_blocked_by_pending_trading_request("disconnecting wallet") {
            return Task::none();
        }
        if self.account_change_blocked_by_active_chase("disconnecting wallet") {
            return Task::none();
        }
        if self.account_change_blocked_by_uncertain_twap("disconnecting wallet") {
            return Task::none();
        }

        let stop_chase_ids: Vec<u64> = self
            .chase_orders
            .iter()
            .filter_map(|(id, chase)| (!chase.lifecycle.is_stopping()).then_some(*id))
            .collect();
        let stop_chase_task = Task::batch(stop_chase_ids.into_iter().map(|id| {
            self.stop_chase_by_id_with_reason(id, "Chase stopped: wallet disconnected", false)
        }));
        let stop_twap_ids: Vec<u64> = self
            .twap_orders
            .iter()
            .filter_map(|(id, twap)| {
                (!twap.status.is_terminal() && !twap.stop_requested).then_some(*id)
            })
            .collect();
        for id in stop_twap_ids {
            let _ = self.stop_twap_with_reason(id, "TWAP stopped: wallet disconnected", false);
        }
        self.invalidate_account_data_requests();
        self.clear_percentage_order_quantity();
        self.bump_account_data_revision();
        self.rotate_account_user_data_stream();
        if let Some(previous_address) = self.connected_address.clone() {
            self.rotate_wallet_detail_user_data_stream_if_open(&previous_address);
        }
        self.connected_address = None;
        self.account_data = None;
        self.account_data_address = None;
        self.pending_order_indicators.clear();
        self.hud_placements.clear();
        self.pending_one_shot_status_requests.clear();
        self.pending_cancel_status_request = None;
        self.pending_move_status_request = None;
        self.clear_pending_move_order_state();
        self.pending_leverage_update = None;
        self.order_leverage_dropdown_open = false;
        self.account_loading = false;
        self.account_refresh_followup_pending = false;
        self.account_reconciliation_required = false;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
        self.account_refresh_retry_due_ms = None;
        self.wallet_key_input.zeroize();
        self.wallet_address_input.clear();
        self.clear_account_scoped_chart_state();
        self.clear_portfolio_income_account_state();
        if self.journal.window_id.is_some() {
            self.journal.clear_active_account_data();
            self.journal.error = Some("Connect an account before loading the journal.".to_string());
        }
        self.persist_config();
        stop_chase_task
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::chart::{HudSelectorKind, OrderOverlay, PositionOverlay, TradeMarker};
    use crate::chart_state::{ChartInstance, ChartSurfaceId};
    use crate::config::{self, AccountProfile};
    use crate::order_execution::{
        MoveOrderKey, OneShotPlacementContext, OrderSurface, PendingLeverageUpdateContext,
        PendingMoveOrderContext, PendingOrderAction, QuickOrderForm,
    };
    use crate::order_update::{
        PendingCancelStatusRequest, PendingMoveStatusRequest, PendingOneShotStatusRequest,
    };
    use crate::signing::ExchangeOrderKind;
    use crate::signing::{ChaseLifecycle, ChaseOrder};
    use crate::timeframe::Timeframe;
    use crate::twap_state::{
        TwapChildOrder, TwapChildStatus, TwapOrder, TwapOrderInit, TwapStatus,
    };
    use std::cell::{Cell, RefCell};
    use std::time::Instant;
    use zeroize::Zeroizing;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

    fn account(secret_id: &str, wallet_address: &str, agent_key: &str) -> AccountProfile {
        AccountProfile {
            secret_id: secret_id.to_string(),
            name: secret_id.to_string(),
            wallet_address: wallet_address.to_string(),
            agent_key: sensitive_string(agent_key).into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }
    }

    fn terminal_with_encrypted_account(
        wallet_address: &str,
        agent_key: &str,
        unlocked: bool,
    ) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.desktop_notifications = false;
        terminal.accounts = vec![account("acct-a", wallet_address, agent_key)];
        terminal.active_account_index = 0;
        terminal.wallet_address_input = wallet_address.to_string();
        terminal.wallet_key_input = sensitive_string(agent_key);
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secret_password = sensitive_string("test-password");
        terminal.encrypted_secrets = Some(
            config::encrypt_secrets(
                &config::SecretPayload::from_credentials(&terminal.accounts, "", ""),
                &terminal.encrypted_secret_password,
            )
            .expect("test encrypted payload"),
        );
        terminal.encrypted_secrets_unlocked = unlocked;
        terminal.secret_store_status = None;
        terminal.secret_migration_save_blocked = false;
        terminal.config_save_due_at = None;
        terminal
    }

    fn pending_one_shot_status_request(account_address: &str) -> PendingOneShotStatusRequest {
        PendingOneShotStatusRequest::new(
            7,
            &OneShotPlacementContext {
                account_address: account_address.to_string(),
                cloid: "0x00000000000000000000000000000000".to_string(),
                surface: OrderSurface::Ticket,
                symbol_key: "BTC".to_string(),
                order_kind: ExchangeOrderKind::Limit,
            },
        )
    }

    fn pending_cancel_status_request(account_address: &str) -> PendingCancelStatusRequest {
        PendingCancelStatusRequest::new(7, account_address.to_string(), 42, "BTC".to_string())
    }

    fn pending_move_status_request(account_address: &str) -> PendingMoveStatusRequest {
        PendingMoveStatusRequest::new(
            8,
            account_address.to_string(),
            42,
            "BTC".to_string(),
            "100".to_string(),
        )
    }

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
            duration: std::time::Duration::from_secs(60),
            slice_count: 1,
            now: Instant::now(),
            started_at_ms: TradingTerminal::now_ms(),
        })
    }

    fn status_check_twap(id: u64, account_address: &str) -> TwapOrder {
        let mut twap = twap_order(id, account_address);
        twap.status_check_cloid = Some("0xabc".to_string());
        twap
    }

    fn reconciliation_twap(id: u64, account_address: &str) -> TwapOrder {
        let mut twap = twap_order(id, account_address);
        let now = Instant::now();
        twap.child_orders.push(TwapChildOrder {
            index: 1,
            requested_at: now,
            planned_size: 1.0,
            limit_price: 50_000.0,
            oid: Some(1001),
            cloid: Some("0xabc".to_string()),
            status: TwapChildStatus::AwaitingReconciliation,
            exchange_summary: "filled; waiting for account fills".to_string(),
            filled_size: 0.0,
            avg_price: None,
            fee: 0.0,
            retry_count: 0,
        });
        twap.reconciliation_deadline = Some(now + std::time::Duration::from_secs(30));
        twap
    }

    fn deadline_only_reconciliation_twap(id: u64, account_address: &str) -> TwapOrder {
        let mut twap = twap_order(id, account_address);
        twap.reconciliation_deadline = Some(Instant::now() + std::time::Duration::from_secs(30));
        twap
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
    fn connect_wallet_blocks_cross_account_reconnect_while_order_action_pending() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked reconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before connecting a wallet")
        );
    }

    #[test]
    fn connect_wallet_blocks_invalid_address_while_chase_is_active() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = "not-a-wallet-address".to_string();
        terminal.account_error = Some("existing account error".to_string());
        terminal.chase_orders.insert(42, chase_order(TEST_ACCOUNT));

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, "not-a-wallet-address");
        assert_eq!(
            terminal.account_error.as_deref(),
            Some("existing account error")
        );
        assert!(terminal.chase_orders.contains_key(&42));
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked invalid connect should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("Stop active chase orders"));
        assert!(
            !terminal
                .toasts
                .iter()
                .any(|toast| toast.message == "Invalid wallet address")
        );
    }

    #[test]
    fn connect_wallet_blocks_invalid_address_while_twap_is_active() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = "not-a-wallet-address".to_string();
        terminal.account_error = Some("existing account error".to_string());
        terminal.twap_orders.insert(7, twap_order(7, TEST_ACCOUNT));

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, "not-a-wallet-address");
        assert_eq!(
            terminal.account_error.as_deref(),
            Some("existing account error")
        );
        assert!(
            !terminal
                .twap_orders
                .get(&7)
                .map(|twap| twap.stop_requested)
                .unwrap_or(false)
        );
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked invalid connect should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("Stop active TWAP orders"));
        assert!(
            !terminal
                .toasts
                .iter()
                .any(|toast| toast.message == "Invalid wallet address")
        );
    }

    #[test]
    fn invalid_wallet_clears_account_scoped_refresh_state() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = "not-a-wallet-address".to_string();
        terminal.account_loading = true;
        terminal.account_refresh_followup_pending = true;
        terminal.account_reconciliation_required = true;
        terminal.account_refresh_backoff_until_ms = Some(TradingTerminal::now_ms() + 60_000);
        terminal.account_refresh_retry_due_ms = terminal.account_refresh_backoff_until_ms;
        terminal.account_error = Some("old account error".to_string());
        let account_context = terminal.current_account_data_request_context();
        let portfolio_request_id = terminal.portfolio.begin_refresh();
        terminal.portfolio.last_error = Some("old portfolio error".to_string());
        let income_request_id = terminal.income.begin_refresh();
        terminal.income.last_error = Some("old income error".to_string());
        terminal.last_income_alert_time = Some(123);
        seed_account_scoped_chart_state(&mut terminal);

        let _task = terminal.connect_wallet();

        assert!(terminal.connected_address.is_none());
        assert!(
            !terminal.account_data_request_generation_is_current(TEST_ACCOUNT, account_context)
        );
        assert!(!terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(!terminal.account_reconciliation_required);
        assert!(terminal.account_refresh_backoff_until_ms.is_none());
        assert!(terminal.account_refresh_retry_due_ms.is_none());
        assert_eq!(
            terminal.account_error.as_deref(),
            Some("Invalid wallet address")
        );
        assert_ne!(terminal.portfolio.refresh_request_id, portfolio_request_id);
        assert!(!terminal.portfolio.loading);
        assert!(terminal.portfolio.last_error.is_none());
        assert_ne!(terminal.income.refresh_request_id, income_request_id);
        assert!(!terminal.income.loading);
        assert!(terminal.income.last_error.is_none());
        assert!(terminal.last_income_alert_time.is_none());
        assert_account_scoped_chart_state_cleared(&terminal);
    }

    #[test]
    fn connect_wallet_blocks_cross_account_reconnect_while_chase_is_active() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal.chase_orders.insert(42, chase_order(TEST_ACCOUNT));

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.chase_orders.contains_key(&42));
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked reconnect should toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("Stop active chase orders"));
    }

    #[test]
    fn connect_wallet_allows_same_account_reconnect_while_chase_is_active() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_ascii_uppercase();
        terminal.chase_orders.insert(42, chase_order(TEST_ACCOUNT));
        terminal.account_loading = true;
        let stale_context = terminal.current_account_data_request_context();
        let previous_stream_generation = terminal.account_user_data_stream_generation;

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.chase_orders.contains_key(&42));
        assert!(terminal.account_loading);
        assert!(!terminal.account_data_request_generation_is_current(TEST_ACCOUNT, stale_context));
        assert_ne!(
            terminal.account_user_data_stream_generation,
            previous_stream_generation
        );
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn connect_wallet_marks_account_reconciliation_until_snapshot_loads() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", TEST_ACCOUNT, "agent-key")];
        terminal.active_account_index = 0;
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.account_reconciliation_required = false;

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
    }

    #[test]
    fn connect_wallet_clears_pending_connect_bridge_flag() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", TEST_ACCOUNT, "agent-key")];
        terminal.active_account_index = 0;
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        // Simulate the in-flight switch/boot state where the summary is showing
        // the connecting skeleton.
        terminal.account_connect_pending = true;

        let _task = terminal.connect_wallet();

        // Processing the connect settles the real address and retires the bridge.
        assert!(!terminal.account_connect_pending);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
    }

    #[test]
    fn disconnect_wallet_clears_pending_connect_bridge_flag() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.account_connect_pending = true;

        let _task = terminal.disconnect_wallet();

        assert!(!terminal.account_connect_pending);
        assert_eq!(terminal.connected_address, None);
    }

    #[test]
    fn connect_wallet_blocks_same_account_reconnect_while_leverage_update_is_pending() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_ascii_uppercase();
        terminal.pending_leverage_update = Some(PendingLeverageUpdateContext {
            address: TEST_ACCOUNT.to_string(),
            symbol_key: "BTC".to_string(),
            display: "BTC".to_string(),
            asset: 0,
            dex: None,
            is_cross: true,
            leverage: 5,
        });

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.pending_leverage_update.is_some());
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked reconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before connecting a wallet")
        );
    }

    #[test]
    fn connect_wallet_blocks_reconnect_while_one_shot_status_is_pending() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_ascii_uppercase();
        terminal
            .insert_pending_one_shot_status_request(pending_one_shot_status_request(TEST_ACCOUNT));

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.has_pending_one_shot_status_requests_for_test());
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked reconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before connecting a wallet")
        );
    }

    #[test]
    fn connect_wallet_locked_encrypted_credentials_preserves_previous_wallet_binding() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", false);
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal.twap_orders.insert(7, {
            let mut twap = twap_order(7, TEST_ACCOUNT);
            twap.stop_requested = false;
            twap
        });
        let original_encrypted = terminal.encrypted_secrets.clone();
        let profile_key_allocation = terminal.accounts[0].agent_key.as_ptr();
        let input_key_allocation = terminal.wallet_key_input.as_str().as_ptr();

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts[0].wallet_address, TEST_ACCOUNT);
        assert_eq!(terminal.wallet_key_input.as_str(), "old-account-agent-key");
        assert_eq!(
            terminal.accounts[0].agent_key.as_str(),
            "old-account-agent-key"
        );
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            profile_key_allocation
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            input_key_allocation
        );
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        assert!(!terminal.account_loading);
        assert!(
            !terminal
                .twap_orders
                .get(&7)
                .map(|twap| twap.stop_requested)
                .unwrap_or(false)
        );
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Unlock encrypted credentials"));
        assert!(message.contains("Wallet was not connected"));
    }

    #[test]
    fn connect_wallet_rebinding_is_rejected_while_config_save_is_in_flight() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", true);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.config_save_in_flight = true;
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts[0].wallet_address, TEST_ACCOUNT);
        assert_eq!(terminal.wallet_key_input.as_str(), "old-account-agent-key");
        assert_eq!(
            terminal.accounts[0].agent_key.as_str(),
            "old-account-agent-key"
        );
        assert!(terminal.config_save_in_flight);
        assert!(!terminal.account_loading);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Config save failed"));
        assert!(message.contains("Wallet was not connected"));
    }

    #[test]
    fn connect_wallet_config_save_failure_redacts_status() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", true);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        let keychain_called = Cell::new(false);

        let _task = terminal.connect_wallet_with_hooks(
            |_cfg| Err("write failed: api_key=connect-secret".to_string()),
            |_terminal, _accounts, _removed_profile_secret_id| {
                keychain_called.set(true);
                true
            },
        );

        assert!(!keychain_called.get());
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("api_key=<redacted>"));
        assert!(!message.contains("connect-secret"));
        assert!(message.contains("Wallet was not connected"));
    }

    #[test]
    fn connect_wallet_installed_snapshot_error_preserves_failure_behavior_pending_policy() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", true);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        let keychain_called = Cell::new(false);
        let profile_key_allocation = terminal.accounts[0].agent_key.as_ptr();
        let input_key_allocation = terminal.wallet_key_input.as_str().as_ptr();

        let _task = terminal.connect_wallet_with_hooks(
            |snapshot| {
                assert_eq!(snapshot.accounts[0].wallet_address, OTHER_ACCOUNT);
                Err(config::installed_config_save_error_for_test(
                    "sync failed: api_key=connect-secret",
                ))
            },
            |_terminal, _accounts, _removed_profile_secret_id| {
                keychain_called.set(true);
                true
            },
        );

        assert!(!keychain_called.get());
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts[0].wallet_address, TEST_ACCOUNT);
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            profile_key_allocation
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            input_key_allocation
        );
        assert!(!terminal.account_loading);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Config save failed"));
        assert!(message.contains("api_key=<redacted>"));
        assert!(!message.contains("connect-secret"));
        assert!(message.contains("Wallet was not connected"));
    }

    #[test]
    fn connect_wallet_os_keychain_failure_rolls_back_saved_metadata_and_does_not_connect() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", true);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        let saved_snapshots = RefCell::new(Vec::<config::KeroseneConfig>::new());
        let keychain_called = Cell::new(false);
        let profile_key_allocation = terminal.accounts[0].agent_key.as_ptr();
        let input_key_allocation = terminal.wallet_key_input.as_str().as_ptr();

        let _task = terminal.connect_wallet_with_hooks(
            |cfg| {
                saved_snapshots.borrow_mut().push(cfg.clone());
                Ok(())
            },
            |terminal, accounts, removed_profile_secret_id| {
                keychain_called.set(true);
                assert_eq!(removed_profile_secret_id, "acct-a");
                assert_eq!(accounts.len(), 1);
                assert_eq!(accounts[0].wallet_address, OTHER_ACCOUNT);
                assert!(accounts[0].agent_key.trim().is_empty());
                terminal.secret_migration_save_blocked = true;
                terminal.secret_store_status =
                    Some(("Keychain update failed: denied".into(), true));
                false
            },
        );

        assert!(keychain_called.get());
        let saved_snapshots = saved_snapshots.borrow();
        assert_eq!(saved_snapshots.len(), 2);
        assert_eq!(saved_snapshots[0].accounts[0].wallet_address, OTHER_ACCOUNT);
        assert!(saved_snapshots[0].accounts[0].agent_key.trim().is_empty());
        assert_eq!(saved_snapshots[1].accounts[0].wallet_address, TEST_ACCOUNT);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert_eq!(terminal.accounts[0].wallet_address, TEST_ACCOUNT);
        assert_eq!(terminal.wallet_key_input.as_str(), "old-account-agent-key");
        assert_eq!(
            terminal.accounts[0].agent_key.as_str(),
            "old-account-agent-key"
        );
        assert_eq!(
            terminal.accounts[0].agent_key.as_ptr(),
            profile_key_allocation
        );
        assert_eq!(
            terminal.wallet_key_input.as_str().as_ptr(),
            input_key_allocation
        );
        assert!(terminal.secret_migration_save_blocked);
        assert!(!terminal.account_loading);
        assert!(!terminal.account_reconciliation_required);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Keychain update failed: denied"));
        assert!(message.contains("Wallet was not connected"));
        assert!(!message.contains("rollback failed"));
    }

    #[test]
    fn connect_wallet_rollback_save_failure_redacts_status() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", true);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        let save_count = Cell::new(0);

        let _task = terminal.connect_wallet_with_hooks(
            |_cfg| {
                let count = save_count.get();
                save_count.set(count + 1);
                if count == 0 {
                    Ok(())
                } else {
                    Err("rollback failed: signature=rollback-secret".to_string())
                }
            },
            |terminal, _accounts, _removed_profile_secret_id| {
                terminal.secret_store_status = Some((
                    "Keychain update failed: auth_token=keychain-secret".to_string(),
                    true,
                ));
                false
            },
        );

        assert_eq!(save_count.get(), 2);
        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("auth_token=<redacted>"));
        assert!(message.contains("signature=<redacted>"));
        assert!(!message.contains("keychain-secret"));
        assert!(!message.contains("rollback-secret"));
        assert!(message.contains("Wallet was not connected"));
    }

    #[test]
    fn connect_wallet_blocks_account_change_while_twap_status_check_is_pending() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal
            .twap_orders
            .insert(7, status_check_twap(7, TEST_ACCOUNT));

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, OTHER_ACCOUNT);
        let twap = terminal.twap_orders.get(&7).expect("twap");
        assert!(!twap.stop_requested);
        assert!(twap.status_check_cloid.is_some());
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked connect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("TWAP order status and fill reconciliation")
        );
    }

    #[test]
    fn connect_wallet_blocks_account_change_while_twap_reconciliation_deadline_is_pending() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        terminal
            .twap_orders
            .insert(7, deadline_only_reconciliation_twap(7, TEST_ACCOUNT));

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, OTHER_ACCOUNT);
        let twap = terminal.twap_orders.get(&7).expect("twap");
        assert!(!twap.stop_requested);
        assert!(twap.reconciliation_deadline.is_some());
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked connect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("TWAP order status and fill reconciliation")
        );
    }

    #[test]
    fn connect_wallet_address_change_clears_agent_key_after_secret_persistence_succeeds() {
        let mut terminal =
            terminal_with_encrypted_account(TEST_ACCOUNT, "old-account-agent-key", true);
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(OTHER_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, OTHER_ACCOUNT);
        assert_eq!(terminal.accounts[0].wallet_address, OTHER_ACCOUNT);
        assert_eq!(terminal.wallet_key_input.as_str(), "");
        assert_eq!(terminal.accounts[0].agent_key.as_str(), "");
        let payload = config::decrypt_secrets(
            terminal
                .encrypted_secrets
                .as_ref()
                .expect("encrypted secrets should be rewritten"),
            &terminal.encrypted_secret_password,
        )
        .expect("encrypted secrets should decrypt");
        assert_eq!(payload.profile_agent_key("acct-a"), None);
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(message.contains("Agent key cleared"));
    }

    #[test]
    fn connect_wallet_does_not_commit_hyperdash_draft_key() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("ghost-a", TEST_ACCOUNT, "")];
        terminal.active_account_index = 0;
        terminal
            .ghost_account_secret_ids
            .insert("ghost-a".to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.hyperdash_api_key = sensitive_string("saved-hyperdash-key");
        terminal.hyperdash_key_input = sensitive_string("draft-hyperdash-key");
        terminal.hyperdash_key_generation = 4;

        let _task = terminal.connect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.hyperdash_api_key.as_str(), "saved-hyperdash-key");
        assert_eq!(terminal.hyperdash_key_input.as_str(), "draft-hyperdash-key");
        assert_eq!(terminal.hyperdash_key_generation, 4);
    }

    #[test]
    fn connect_wallet_does_not_rewrite_terminal_twaps() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", TEST_ACCOUNT, "")];
        terminal.active_account_index = 0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = OTHER_ACCOUNT.to_string();
        let mut completed = twap_order(7, TEST_ACCOUNT);
        completed.status = TwapStatus::Completed;
        terminal.twap_orders.insert(7, completed);

        let _task = terminal.connect_wallet();

        let twap = terminal.twap_orders.get(&7).expect("twap");
        assert_eq!(twap.status, TwapStatus::Completed);
        assert!(!twap.stop_requested);
        assert_eq!(twap.stop_reason, None);
    }

    #[test]
    fn disconnect_blocks_pending_indicators_and_preserves_market_pulse() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        let pending_id = terminal.add_pending_market_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        assert!(pending_id.is_some());
        terminal.pending_move_order_contexts.insert(
            MoveOrderKey::new("BTC", 42),
            PendingMoveOrderContext::new(
                0,
                TEST_ACCOUNT.to_string(),
                "100",
                Zeroizing::new("move-agent".to_string()),
            )
            .expect("move context"),
        );
        terminal.active_move_order_drag = Some(MoveOrderKey::new("BTC", 42));
        assert!(
            terminal
                .charts
                .get(&1)
                .expect("chart")
                .chart
                .hud_order_animation_active()
        );

        let _task = terminal.disconnect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(!terminal.pending_order_indicators.is_empty());
        assert!(!terminal.pending_move_order_contexts.is_empty());
        assert_eq!(
            terminal.active_move_order_drag,
            Some(MoveOrderKey::new("BTC", 42))
        );
        let chart = &terminal.charts.get(&1).expect("chart").chart;
        assert!(chart.hud_order_animation_active());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked disconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before disconnecting wallet")
        );
    }

    #[test]
    fn disconnect_blocks_pending_one_shot_status_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal
            .insert_pending_one_shot_status_request(pending_one_shot_status_request(TEST_ACCOUNT));

        let _task = terminal.disconnect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.has_pending_one_shot_status_requests_for_test());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked disconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before disconnecting wallet")
        );
    }

    #[test]
    fn disconnect_blocks_pending_cancel_status_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.pending_cancel_status_request = Some(pending_cancel_status_request(TEST_ACCOUNT));

        let _task = terminal.disconnect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.pending_cancel_status_request.is_some());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked disconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before disconnecting wallet")
        );
    }

    #[test]
    fn disconnect_blocks_pending_move_status_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.pending_move_status_request = Some(pending_move_status_request(TEST_ACCOUNT));

        let _task = terminal.disconnect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert!(terminal.pending_move_status_request.is_some());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked disconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("pending trading requests to finish before disconnecting wallet")
        );
    }

    #[test]
    fn disconnect_wallet_blocks_active_chase() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.chase_orders.insert(42, chase_order(TEST_ACCOUNT));

        let _task = terminal.disconnect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        assert!(terminal.chase_orders.contains_key(&42));
        let chase = terminal.chase_orders.get(&42).expect("chase");
        assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
        assert_eq!(chase.current_oid, Some(1001));
        assert!(!terminal.account_loading);
        assert!(terminal.account_data_address.is_none());
        let toast = terminal
            .toasts
            .last()
            .expect("blocked disconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("Stop active chase orders and wait for cancellation")
        );
    }

    #[test]
    fn disconnect_clears_account_scoped_refresh_state() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.account_loading = true;
        terminal.account_refresh_followup_pending = true;
        terminal.account_reconciliation_required = true;
        terminal.account_refresh_backoff_until_ms = Some(TradingTerminal::now_ms() + 60_000);
        terminal.account_refresh_retry_due_ms = terminal.account_refresh_backoff_until_ms;
        terminal.account_error = Some("old account error".to_string());
        let account_context = terminal.current_account_data_request_context();
        let portfolio_request_id = terminal.portfolio.begin_refresh();
        terminal.portfolio.last_error = Some("old portfolio error".to_string());
        let income_request_id = terminal.income.begin_refresh();
        terminal.income.last_error = Some("old income error".to_string());
        terminal.last_income_alert_time = Some(123);
        seed_account_scoped_chart_state(&mut terminal);

        let _task = terminal.disconnect_wallet();

        assert!(terminal.connected_address.is_none());
        assert!(
            !terminal.account_data_request_generation_is_current(TEST_ACCOUNT, account_context)
        );
        assert!(!terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(!terminal.account_reconciliation_required);
        assert!(terminal.account_refresh_backoff_until_ms.is_none());
        assert!(terminal.account_refresh_retry_due_ms.is_none());
        assert!(terminal.account_error.is_none());
        assert_ne!(terminal.portfolio.refresh_request_id, portfolio_request_id);
        assert!(!terminal.portfolio.loading);
        assert!(terminal.portfolio.last_error.is_none());
        assert_ne!(terminal.income.refresh_request_id, income_request_id);
        assert!(!terminal.income.loading);
        assert!(terminal.income.last_error.is_none());
        assert!(terminal.last_income_alert_time.is_none());
        assert_account_scoped_chart_state_cleared(&terminal);
    }

    #[test]
    fn disconnect_wallet_blocks_twap_awaiting_fill_reconciliation() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal
            .twap_orders
            .insert(7, reconciliation_twap(7, TEST_ACCOUNT));

        let _task = terminal.disconnect_wallet();

        assert_eq!(terminal.connected_address.as_deref(), Some(TEST_ACCOUNT));
        assert_eq!(terminal.wallet_address_input, TEST_ACCOUNT);
        let twap = terminal.twap_orders.get(&7).expect("twap");
        assert!(!twap.stop_requested);
        assert!(twap.has_status_unknown_child());
        assert!(!terminal.account_loading);
        let toast = terminal
            .toasts
            .last()
            .expect("blocked disconnect should toast");
        assert!(toast.is_error);
        assert!(
            toast
                .message
                .contains("TWAP order status and fill reconciliation")
        );
    }

    #[test]
    fn disconnect_wallet_does_not_rewrite_terminal_twaps() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        let mut completed = twap_order(7, TEST_ACCOUNT);
        completed.status = TwapStatus::Completed;
        terminal.twap_orders.insert(7, completed);

        let _task = terminal.disconnect_wallet();

        let twap = terminal.twap_orders.get(&7).expect("twap");
        assert_eq!(twap.status, TwapStatus::Completed);
        assert!(!twap.stop_requested);
        assert_eq!(twap.stop_reason, None);
    }
}
