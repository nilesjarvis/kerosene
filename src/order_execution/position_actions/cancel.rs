use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::{CancelIntent, OrderSurface, cancel_order_task};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn execute_cancel(&mut self, coin: &str, oid: u64) -> Task<Message> {
        let _theme = self.theme();
        // The chart suppresses interaction on a Cancelling line, but the
        // orders table and queued clicks can still re-dispatch; a duplicate
        // cancel surfaces a spurious red error after the first one lands.
        if self.has_pending_cancel_indicator(oid) {
            self.order_status = Some(("Cancel already pending for this order".into(), true));
            return Task::none();
        }
        if self.reject_if_pending_trading_request("cancelling orders") {
            return Task::none();
        }
        let Some((key, account_address)) = self.order_signing_context() else {
            return Task::none();
        };
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh open orders before cancelling".into(),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("cancelling", "open orders") {
            return Task::none();
        }

        let Some(account_data) = self.account_data_for_order_account(&account_address) else {
            self.order_status = Some((
                "No account data available; refresh before cancelling".into(),
                true,
            ));
            return Task::none();
        };
        let now_ms = Self::now_ms();
        if !account_data.is_fresh_for_open_order_action_for_symbol(coin, now_ms) {
            let age_label = account_data
                .open_order_action_snapshot_age_ms_for_symbol(coin, now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            self.order_status = Some((
                format!("Open orders are stale ({age_label}); refresh before cancelling orders"),
                true,
            ));
            return self.refresh_account_data();
        }
        if !account_data.completeness.open_orders_complete {
            self.order_status = Some((
                "Open orders are incomplete; refresh before cancelling".into(),
                true,
            ));
            return self.refresh_account_data();
        }
        let Some(order) = account_data
            .open_orders
            .iter()
            .find(|order| order.oid == oid && order.coin == coin)
            .cloned()
        else {
            self.order_status = Some(("Order no longer exists".into(), true));
            return Task::none();
        };

        let prepared = match self.prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: coin.to_string(),
            oid,
        }) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        let pending_indicator_id =
            self.add_pending_order_cancellation_indicator(account_address.clone(), &order);

        self.order_status = Some(("Cancelling order...".into(), false));
        cancel_order_task(key, prepared.asset, prepared.oid, move |result| {
            Message::CancelResult {
                account_address: account_address.into(),
                pending_indicator_id,
                result: Box::new(result),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
        SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::app_state::{TradingTerminal, sensitive_string};
    use crate::config::AccountProfile;
    use crate::order_execution::{
        MoveOrderKey, OneShotPlacementContext, OrderSurface, PendingLeverageUpdateContext,
        PendingMoveOrderContext, PendingNukeExecution, PendingOrderAction,
    };
    use crate::order_update::PendingOneShotStatusRequest;
    use crate::signing::ExchangeOrderKind;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn connect_test_account(terminal: &mut TradingTerminal) {
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Account A".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: sensitive_string("").into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }];
        terminal.active_account_index = 0;
    }

    fn btc_symbol() -> ExchangeSymbol {
        ExchangeSymbol {
            key: "BTC".to_string(),
            ticker: "BTC".to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn open_order(oid: u64) -> OpenOrder {
        OpenOrder {
            coin: "BTC".to_string(),
            side: "B".to_string(),
            limit_px: "100".to_string(),
            sz: "1".to_string(),
            oid,
            timestamp: 1,
            reduce_only: Some(false),
            is_trigger: None,
            order_type: None,
            tif: None,
            trigger_px: None,
        }
    }

    fn account_data_with_order(order: OpenOrder) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: vec![order],
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: TradingTerminal::now_ms(),
        }
    }

    fn terminal_with_cancelable_order() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        connect_test_account(&mut terminal);
        terminal.set_committed_agent_key_for_test("agent-key");
        terminal.exchange_symbols = vec![btc_symbol()];
        terminal.set_account_data_for_address_for_test(
            TEST_ACCOUNT,
            account_data_with_order(open_order(42)),
        );
        terminal
    }

    fn one_shot_context() -> OneShotPlacementContext {
        OneShotPlacementContext {
            account_address: TEST_ACCOUNT.to_string(),
            cloid: "0xpending".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Limit,
        }
    }

    fn pending_leverage_update() -> PendingLeverageUpdateContext {
        PendingLeverageUpdateContext {
            address: TEST_ACCOUNT.to_string(),
            symbol_key: "BTC".to_string(),
            display: "BTC".to_string(),
            asset: 0,
            dex: None,
            is_cross: true,
            leverage: 10,
        }
    }

    fn pending_move_context() -> PendingMoveOrderContext {
        PendingMoveOrderContext::new(
            TEST_ACCOUNT.to_string(),
            sensitive_string("move-agent").into_zeroizing(),
        )
        .expect("move context")
    }

    fn assert_cancel_waits_for_pending_trading_request(mut terminal: TradingTerminal) {
        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert!(terminal.pending_order_indicators.is_empty());
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some((
                "Wait for pending trading requests to finish before cancelling orders",
                true
            ))
        );
    }

    #[test]
    fn execute_cancel_creates_cancelling_indicator() {
        let mut terminal = terminal_with_cancelable_order();

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(terminal.has_pending_cancel_indicator(42));
        let indicator = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("cancel indicator");
        assert_eq!(indicator.account_address, TEST_ACCOUNT);
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Cancelling order...", false))
        );
    }

    #[test]
    fn duplicate_cancel_is_gated_while_indicator_pending() {
        let mut terminal = terminal_with_cancelable_order();

        let _task = terminal.execute_cancel("BTC", 42);
        assert_eq!(terminal.pending_order_indicators.len(), 1);

        let _task = terminal.execute_cancel("BTC", 42);

        assert_eq!(terminal.pending_order_indicators.len(), 1);
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Cancel already pending for this order", true))
        );
    }

    #[test]
    fn cancel_of_other_oid_waits_for_pending_cancel() {
        let mut terminal = terminal_with_cancelable_order();
        terminal
            .account_data
            .as_mut()
            .expect("account data")
            .open_orders
            .push(open_order(43));

        let _task = terminal.execute_cancel("BTC", 42);
        let _task = terminal.execute_cancel("BTC", 43);

        assert_eq!(terminal.pending_order_indicators.len(), 1);
        assert!(!terminal.has_pending_cancel_indicator(43));
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some((
                "Wait for pending trading requests to finish before cancelling orders",
                true
            ))
        );
    }

    #[test]
    fn execute_cancel_waits_for_pending_order_action() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        assert_cancel_waits_for_pending_trading_request(terminal);
    }

    #[test]
    fn execute_cancel_waits_for_pending_one_shot_status() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.pending_one_shot_status_request =
            Some(PendingOneShotStatusRequest::new(7, &one_shot_context()));

        assert_cancel_waits_for_pending_trading_request(terminal);
    }

    #[test]
    fn execute_cancel_waits_for_pending_leverage_update() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.pending_leverage_update = Some(pending_leverage_update());

        assert_cancel_waits_for_pending_trading_request(terminal);
    }

    #[test]
    fn execute_cancel_waits_for_pending_nuke_execution() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.pending_nuke_execution = Some(PendingNukeExecution::new(7, 1, 0));

        assert_cancel_waits_for_pending_trading_request(terminal);
    }

    #[test]
    fn execute_cancel_waits_for_pending_move_context() {
        let mut terminal = terminal_with_cancelable_order();
        terminal
            .pending_move_order_contexts
            .insert(MoveOrderKey::new("ETH", 43), pending_move_context());

        assert_cancel_waits_for_pending_trading_request(terminal);
    }

    #[test]
    fn execute_cancel_refuses_missing_open_order_snapshot() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.account_data = None;

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("No account data available; refresh before cancelling", true))
        );
    }

    #[test]
    fn execute_cancel_refuses_mismatched_account_snapshot_owner() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.account_data_address =
            Some("0xdef0000000000000000000000000000000000000".to_string());

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("No account data available; refresh before cancelling", true))
        );
    }

    #[test]
    fn execute_cancel_refuses_stale_open_order_snapshot_and_refreshes() {
        let mut terminal = terminal_with_cancelable_order();
        terminal
            .account_data
            .as_mut()
            .expect("account data")
            .fetched_at_ms = 1;

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert!(terminal.account_loading);
        let (message, is_error) = terminal.order_status.as_ref().expect("order status");
        assert!(*is_error);
        assert!(message.contains("Open orders are stale"));
        assert!(message.contains("refresh before cancelling orders"));
    }

    #[test]
    fn execute_cancel_does_not_treat_positions_refresh_as_open_order_freshness() {
        let mut terminal = terminal_with_cancelable_order();
        let now_ms = TradingTerminal::now_ms();
        let account_data = terminal.account_data.as_mut().expect("account data");
        account_data.fetched_at_ms = 1;
        account_data.mark_positions_fetched_at(now_ms);

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert!(terminal.account_loading);
        let (message, is_error) = terminal.order_status.as_ref().expect("order status");
        assert!(*is_error);
        assert!(message.contains("Open orders are stale"));
        assert!(message.contains("refresh before cancelling orders"));
    }

    #[test]
    fn execute_cancel_does_not_treat_other_dex_open_orders_as_fresh() {
        let mut terminal = terminal_with_cancelable_order();
        let now_ms = TradingTerminal::now_ms();
        let stale_ms = now_ms.saturating_sub(AccountData::POSITION_ACTION_MAX_AGE_MS + 1_000);
        let account_data = terminal.account_data.as_mut().expect("account data");
        account_data.fetched_at_ms = stale_ms;
        account_data.mark_open_orders_fetched_at_for_dex("flx", now_ms);

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert!(terminal.account_loading);
        let (message, is_error) = terminal.order_status.as_ref().expect("order status");
        assert!(*is_error);
        assert!(message.contains("Open orders are stale"));
        assert!(message.contains("refresh before cancelling orders"));
    }

    #[test]
    fn execute_cancel_refuses_incomplete_open_orders_and_refreshes() {
        let mut terminal = terminal_with_cancelable_order();
        terminal
            .account_data
            .as_mut()
            .expect("account data")
            .completeness
            .open_orders_complete = false;

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert!(terminal.account_loading);
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some((
                "Open orders are incomplete; refresh before cancelling",
                true
            ))
        );
    }

    #[test]
    fn execute_cancel_refuses_pending_account_reconciliation() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.account_reconciliation_required = true;

        let _task = terminal.execute_cancel("BTC", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some((
                "Account refresh pending; wait for fresh open orders before cancelling",
                true
            ))
        );
    }

    #[test]
    fn execute_cancel_refuses_oid_without_matching_coin() {
        let mut terminal = terminal_with_cancelable_order();
        terminal.exchange_symbols.push(ExchangeSymbol {
            key: "ETH".to_string(),
            ticker: "ETH".to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 1,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 50,
            only_isolated: false,
            market_type: MarketType::Perp,
            outcome: None,
        });

        let _task = terminal.execute_cancel("ETH", 42);

        assert!(!terminal.has_pending_cancel_indicator(42));
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            Some(("Order no longer exists", true))
        );
    }
}
