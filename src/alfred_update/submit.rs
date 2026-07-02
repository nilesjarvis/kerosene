use crate::alfred_state::{AlfredCommand, AlfredCommandId, alfred_query_is_nuke};
use crate::app_state::TradingTerminal;
use crate::helpers::{finite_value, parse_positive_number, positive_finite_value};
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderOperation, OrderSurface, PlaceIntent, PriceSource, QuantitySource,
    ReduceOnlySource, TicketOrderPlaceIntent, order_size_from_quantity_input,
    reject_if_positions_incomplete_for_action, validate_surface_market_type,
};
use crate::order_update::nuke_confirmation_is_armed;
use crate::signing::{ExchangeOrderKind, OrderKind};
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::Task;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Alfred Command Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn submit_selected_alfred_command(&mut self) -> Task<Message> {
        let commands = self.alfred_filtered_commands();
        let Some(command) = selected_command(&commands, self.alfred.selected_index) else {
            self.push_toast("No Alfred matches".to_string(), true);
            return Task::none();
        };

        self.submit_alfred_command(command.id)
    }

    pub(super) fn submit_alfred_command(&mut self, id: AlfredCommandId) -> Task<Message> {
        if id == AlfredCommandId::NaturalLanguageTrading {
            return self.submit_alfred_trade();
        }
        if id == AlfredCommandId::NukePositions {
            return self.submit_alfred_nuke();
        }
        if id == AlfredCommandId::ClosePosition {
            return self.submit_alfred_close_position();
        }

        let Some(command) = self.alfred_command_by_id(id) else {
            self.push_toast("Alfred command is no longer available".to_string(), true);
            return Task::none();
        };

        if !command.enabled {
            self.push_toast(
                command
                    .disabled_reason
                    .unwrap_or_else(|| "Alfred command is not available yet".to_string()),
                true,
            );
            return Task::none();
        }

        let Some(message) = command.message else {
            self.push_toast("Alfred command is not wired yet".to_string(), true);
            return Task::none();
        };

        self.alfred.close();
        self.update(message)
    }

    fn submit_alfred_trade(&mut self) -> Task<Message> {
        let query = self.alfred.query.clone();
        let Some(draft) = self.alfred_trade_draft(&query) else {
            self.push_toast("Type a trade like 'buy 1k HYPE'".to_string(), true);
            return Task::none();
        };
        if !draft.can_submit() {
            let message = draft
                .error
                .unwrap_or_else(|| "Complete the trade before submitting".to_string());
            self.push_toast(message, true);
            return Task::none();
        }

        let Some(symbol_key) = draft.symbol_key.clone() else {
            self.push_toast("Add a symbol".to_string(), true);
            return Task::none();
        };

        let submit_side = draft.side.map(|side| side.is_buy());
        if !self.alfred_trade_preflight_ready(
            &symbol_key,
            draft.order_kind,
            draft.quantity_is_usd,
            draft.quantity_input(),
            draft.limit_price_input(),
            submit_side,
        ) {
            return Task::none();
        }

        self.alfred.close();
        let switch_task = if self.active_symbol == symbol_key {
            Task::none()
        } else {
            self.switch_active_symbol_internal(symbol_key.clone())
        };
        if self.active_symbol != symbol_key {
            let display = self.display_name_for_symbol(&symbol_key);
            self.push_toast(format!("Cannot trade {display}"), true);
            return switch_task;
        }

        self.order_kind = draft.order_kind;
        self.order_quantity_is_usd = draft.quantity_is_usd;
        self.order_price = match draft.order_kind {
            OrderKind::Limit => draft.limit_price_input().unwrap_or_default(),
            OrderKind::Market => String::new(),
            OrderKind::LimitIoc | OrderKind::Chase | OrderKind::Twap => String::new(),
        };
        self.presets_menu_expanded = false;
        self.handle_order_quantity_changed(draft.quantity_input());
        self.persist_config();

        if let Some(side) = draft.side {
            return Task::batch([switch_task, self.execute_order(side.is_buy())]);
        }

        if draft.order_kind == OrderKind::Chase {
            let display = self.display_name_for_symbol(&symbol_key);
            self.order_status = Some((
                format!("Chase draft ready for {display}: choose CHASE BUY or CHASE SELL"),
                false,
            ));
            self.push_toast(format!("Chase draft ready for {display}"), false);
            return switch_task;
        }

        self.push_toast("Start with buy or sell".to_string(), true);
        switch_task
    }

    fn alfred_trade_preflight_ready(
        &mut self,
        symbol_key: &str,
        order_kind: OrderKind,
        quantity_is_usd: bool,
        quantity: String,
        limit_price: Option<String>,
        submit_is_buy: Option<bool>,
    ) -> bool {
        if quantity_is_usd && self.is_outcome_coin(symbol_key) {
            self.order_status = Some((
                "USD sizing is not supported for outcome markets; use contracts".to_string(),
                true,
            ));
            self.toast_order_status();
            return false;
        }

        let Some(is_buy) = submit_is_buy else {
            return true;
        };

        match order_kind {
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc => self
                .alfred_exchange_order_preflight_ready(
                    symbol_key,
                    order_kind,
                    quantity_is_usd,
                    quantity,
                    limit_price,
                    is_buy,
                ),
            OrderKind::Chase => {
                self.alfred_chase_preflight_ready(symbol_key, quantity_is_usd, quantity)
            }
            OrderKind::Twap => true,
        }
    }

    fn alfred_exchange_order_preflight_ready(
        &mut self,
        symbol_key: &str,
        order_kind: OrderKind,
        quantity_is_usd: bool,
        quantity: String,
        limit_price: Option<String>,
        is_buy: bool,
    ) -> bool {
        if self.reject_if_pending_trading_request("placing an order") {
            self.toast_order_status();
            return false;
        }
        if self.reject_if_account_reconciliation_required("placing an order", "account data") {
            self.toast_order_status();
            return false;
        }
        if self.checked_order_signing_account().is_none() {
            self.toast_order_status();
            return false;
        }

        let exchange_order_kind = match ExchangeOrderKind::try_from(order_kind) {
            Ok(kind) => kind,
            Err(message) => {
                self.order_status = Some((message.into(), true));
                self.toast_order_status();
                return false;
            }
        };
        let intent = Self::ticket_order_place_intent(TicketOrderPlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: symbol_key.to_string(),
            is_buy,
            order_kind: exchange_order_kind,
            price_input: limit_price.unwrap_or_default(),
            quantity_input: quantity,
            quantity_is_usd,
            reduce_only: self.order_reduce_only,
        });

        match self.prepare_place_order(intent) {
            Ok(_) => true,
            Err(message) => {
                self.order_status = Some((message, true));
                self.toast_order_status();
                false
            }
        }
    }

    fn alfred_chase_preflight_ready(
        &mut self,
        symbol_key: &str,
        quantity_is_usd: bool,
        quantity: String,
    ) -> bool {
        if self.active_advanced_order_count() >= MAX_ACTIVE_ADVANCED_ORDERS {
            self.order_status = Some((
                format!(
                    concat!(
                        "Cannot start chase: maximum of {} ",
                        "active advanced orders reached"
                    ),
                    MAX_ACTIVE_ADVANCED_ORDERS
                ),
                true,
            ));
            self.toast_order_status();
            return false;
        }
        if self.reject_if_pending_trading_request("starting a chase") {
            self.toast_order_status();
            return false;
        }
        if self.reject_if_account_reconciliation_required("starting a chase", "account data") {
            self.toast_order_status();
            return false;
        }
        if self.checked_order_signing_account().is_none() {
            self.toast_order_status();
            return false;
        }

        let Some(symbol) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == symbol_key)
            .cloned()
        else {
            self.order_status = Some((format!("Symbol '{symbol_key}' not found"), true));
            self.toast_order_status();
            return false;
        };
        if let Err(error) = self.validate_exchange_symbol_orderable(
            &symbol,
            OrderSurface::Chase.orderability_context_label(),
        ) {
            self.order_status = Some((error, true));
            self.toast_order_status();
            return false;
        }
        if let Err(error) = validate_surface_market_type(
            OrderSurface::Chase,
            OrderOperation::Place,
            symbol.market_type,
        ) {
            self.order_status = Some((error.status_text(), true));
            self.toast_order_status();
            return false;
        }

        let Some(raw_qty) = parse_positive_number(&quantity) else {
            self.order_status = Some(("Invalid quantity".into(), true));
            self.toast_order_status();
            return false;
        };
        let reference_price = if quantity_is_usd {
            let Some(price) = self.resolve_mid_for_symbol(symbol_key) else {
                self.order_status = Some((
                    format!(
                        concat!(
                            "Cannot start USD Chase: no fresh mid price for {}. ",
                            "Wait for market data or enter size in coin units."
                        ),
                        symbol_key
                    ),
                    true,
                ));
                self.toast_order_status();
                return false;
            };
            price
        } else {
            1.0
        };
        if order_size_from_quantity_input(
            raw_qty,
            reference_price,
            quantity_is_usd,
            symbol.sz_decimals,
        )
        .is_none()
        {
            self.order_status = Some(("Invalid quantity for asset precision".into(), true));
            self.toast_order_status();
            return false;
        }

        true
    }

    fn submit_alfred_nuke(&mut self) -> Task<Message> {
        let query = self.alfred.query.clone();
        let Some(command) = self.alfred_command_by_id(AlfredCommandId::NukePositions) else {
            self.push_toast(
                "Type 'nuke' or 'close all' to close open positions".to_string(),
                true,
            );
            return Task::none();
        };

        if !alfred_query_is_nuke(&query) || !command.enabled {
            self.push_toast(
                command
                    .disabled_reason
                    .unwrap_or_else(|| "NUKE is not available".to_string()),
                true,
            );
            return Task::none();
        }

        // Route through the same two-press arming flow as the NUKE button so
        // a single Enter in the palette can never flatten every position.
        let was_armed = nuke_confirmation_is_armed(self.nuke_confirmation.as_ref(), Instant::now());
        let task = self.handle_nuke_positions();
        if was_armed && self.pending_nuke_execution.is_some() {
            // Second press: the nuke is executing; the palette's job is done.
            self.alfred.close();
        } else if let Some((status, is_error)) = self.order_status.clone() {
            // First press armed (or refused to arm); echo the plan where the
            // user is looking and keep the palette open for the confirm press.
            self.push_toast(status, is_error);
        }
        task
    }

    fn submit_alfred_close_position(&mut self) -> Task<Message> {
        let query = self.alfred.query.clone();
        let Some(draft) = self.alfred_close_position_draft(&query) else {
            self.push_toast("Type 'close HYPE' to close a position".to_string(), true);
            return Task::none();
        };
        if !draft.can_submit() {
            self.push_toast(
                draft
                    .error
                    .unwrap_or_else(|| "Complete the close command before submitting".to_string()),
                true,
            );
            return Task::none();
        }

        let Some(coin) = draft.coin else {
            self.push_toast("Add a ticker to close".to_string(), true);
            return Task::none();
        };

        if let Some(task) = self.alfred_close_position_preflight_task(&coin, draft.fraction) {
            return task;
        }

        self.alfred.close();
        self.close_menu_coin = None;
        self.execute_close_position(&coin, draft.fraction, true)
    }

    fn alfred_close_position_preflight_task(
        &mut self,
        coin: &str,
        fraction: f64,
    ) -> Option<Task<Message>> {
        if self.reject_if_pending_trading_request("closing positions") {
            self.toast_order_status();
            return Some(Task::none());
        }
        let Some(account_address) = self.checked_order_signing_account() else {
            self.toast_order_status();
            return Some(Task::none());
        };
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before closing".into(),
                true,
            ));
            self.toast_order_status();
            return Some(Task::none());
        }
        if self.reject_if_account_reconciliation_required("closing", "account data") {
            self.toast_order_status();
            return Some(Task::none());
        }
        if let Some(task) = reject_if_positions_incomplete_for_action(self, "closing positions") {
            self.toast_order_status();
            return Some(task);
        }

        let raw_szi = {
            let Some(account_data) = self.account_data_for_order_account(&account_address) else {
                self.order_status = Some((
                    "No account data available; refresh before closing".into(),
                    true,
                ));
                self.toast_order_status();
                return Some(Task::none());
            };
            let now_ms = Self::now_ms();
            if !account_data.is_fresh_for_position_action(now_ms) {
                let age_label = account_data
                    .position_action_snapshot_age_ms(now_ms)
                    .map(|age| format!("{}s old", age.div_ceil(1000)))
                    .unwrap_or_else(|| "from the future".to_string());
                self.order_status = Some((
                    format!(
                        "Account data is stale ({age_label}); refresh before closing positions"
                    ),
                    true,
                ));
                self.toast_order_status();
                return Some(self.refresh_account_data());
            }

            let Some(position) = account_data
                .clearinghouse
                .asset_positions
                .iter()
                .find(|ap| ap.position.coin == coin)
                .map(|ap| &ap.position)
            else {
                self.order_status = Some((format!("No position found for {coin}"), true));
                self.toast_order_status();
                return Some(Task::none());
            };
            position.szi.clone()
        };

        let Some(close_fraction) =
            positive_finite_value(fraction).filter(|fraction| *fraction <= 1.0)
        else {
            self.order_status = Some(("Close fraction is invalid".into(), true));
            self.toast_order_status();
            return Some(Task::none());
        };
        let Some(position_size) = raw_szi
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(finite_value)
            .filter(|size| size.abs() > 1e-12)
        else {
            self.order_status = Some(("Position size is invalid".into(), true));
            self.toast_order_status();
            return Some(Task::none());
        };

        let intent = PlaceIntent {
            surface: OrderSurface::ClosePosition,
            symbol_key: coin.to_string(),
            is_buy: position_size < 0.0,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: None,
                usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
            },
            quantity_source: QuantitySource::CoinSize {
                size: position_size.abs() * close_fraction,
                invalid_message: "Position size is invalid",
                precision_invalid_message: "Position size is invalid",
            },
            reduce_only_source: ReduceOnlySource::Fixed(true),
        };
        if let Err(message) = self.prepare_place_order(intent) {
            self.order_status = Some((message, true));
            self.toast_order_status();
            return Some(Task::none());
        }

        None
    }
}

fn selected_command(commands: &[AlfredCommand], selected_index: usize) -> Option<&AlfredCommand> {
    let index = selected_index.min(commands.len().checked_sub(1)?);
    commands.get(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
        Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
    use crate::app_state::sensitive_string;
    use crate::config::AccountProfile;
    use crate::order_execution::PendingOrderAction;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TicketSnapshot {
        active_symbol: String,
        active_symbol_display: String,
        order_kind: OrderKind,
        order_quantity: String,
        order_quantity_is_usd: bool,
        order_price: String,
        presets_menu_expanded: bool,
        alfred_open: bool,
    }

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 50,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn spot_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            ticker: ticker.to_string(),
            category: "spot".to_string(),
            display_name: Some(format!("{ticker}/USDC")),
            max_leverage: 1,
            ..symbol(key, MarketType::Spot)
        }
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 66,
                question_id: Some(12),
                question_name: Some("Recurring".to_string()),
                question_description: None,
                question_class: Some("priceBucket".to_string()),
                question_underlying: Some("BTC".to_string()),
                question_expiry: Some("20260520-0600".to_string()),
                question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
                question_period: Some("1d".to_string()),
                question_named_outcomes: vec![67, 68, 69],
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: Some(66),
                bucket_index: Some(0),
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Recurring Named Outcome".to_string(),
                description: "index:0".to_string(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDH".to_string(),
                quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
                encoding: 660,
            }),
            ..symbol(key, MarketType::Outcome)
        }
    }

    fn account_data_with_position(coin: &str, fetched_at_ms: u64) -> AccountData {
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
                asset_positions: vec![AssetPosition {
                    position: Position {
                        coin: coin.to_string(),
                        szi: "1".to_string(),
                        entry_px: "100".to_string(),
                        position_value: "100".to_string(),
                        unrealized_pnl: "0".to_string(),
                        liquidation_px: None,
                        leverage: PositionLeverage {
                            leverage_type: "cross".to_string(),
                            value: 1,
                        },
                        margin_used: "0".to_string(),
                        cum_funding: None,
                    },
                    liquidation_px: None,
                }],
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms,
        }
    }

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
        terminal.set_committed_agent_key_for_test("agent-key");
    }

    fn alfred_close_terminal(fetched_at_ms: u64) -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.set_account_data_for_address_for_test(
            TEST_ACCOUNT,
            account_data_with_position("BTC", fetched_at_ms),
        );
        terminal.account_loading = false;
        terminal.alfred.open = true;
        terminal.alfred.query = "close BTC".to_string();
        terminal.close_menu_coin = Some("BTC".to_string());
        terminal
    }

    fn alfred_trade_terminal() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            symbol("BTC", MarketType::Perp),
            symbol("ETH", MarketType::Perp),
        ];
        terminal.active_symbol = "BTC".to_string();
        terminal.active_symbol_display = "BTC".to_string();
        terminal.order_kind = OrderKind::Limit;
        terminal.order_quantity = "old-size".to_string();
        terminal.order_quantity_is_usd = true;
        terminal.order_price = "old-price".to_string();
        terminal.presets_menu_expanded = true;
        terminal.alfred.open = true;
        terminal
    }

    fn add_mid(terminal: &mut TradingTerminal, symbol: &str, mid: f64) {
        terminal.all_mids.insert(symbol.to_string(), mid);
        terminal
            .all_mids_updated_at_ms
            .insert(symbol.to_string(), TradingTerminal::now_ms());
    }

    fn ticket_snapshot(terminal: &TradingTerminal) -> TicketSnapshot {
        TicketSnapshot {
            active_symbol: terminal.active_symbol.clone(),
            active_symbol_display: terminal.active_symbol_display.clone(),
            order_kind: terminal.order_kind,
            order_quantity: terminal.order_quantity.clone(),
            order_quantity_is_usd: terminal.order_quantity_is_usd,
            order_price: terminal.order_price.clone(),
            presets_menu_expanded: terminal.presets_menu_expanded,
            alfred_open: terminal.alfred.open,
        }
    }

    fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
        match terminal.order_status.as_ref() {
            Some((message, is_error)) => (message.as_str(), *is_error),
            None => panic!("missing order status"),
        }
    }

    #[test]
    fn alfred_trade_rejects_usd_sizing_for_outcome_markets() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#660")];
        terminal.active_symbol = "#660".to_string();
        terminal.order_quantity = "old".to_string();
        terminal.order_quantity_is_usd = false;
        terminal.alfred.open = true;
        terminal.alfred.query = "buy $10 #660".to_string();

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(
            terminal.order_status,
            Some((
                "USD sizing is not supported for outcome markets; use contracts".to_string(),
                true
            ))
        );
        assert_eq!(terminal.order_quantity, "old");
        assert!(!terminal.order_quantity_is_usd);
    }

    #[test]
    fn alfred_trade_outcome_usd_rejection_does_not_switch_or_mutate_ticket() {
        let mut terminal = alfred_trade_terminal();
        terminal.exchange_symbols.push(outcome_symbol("#660"));
        terminal.alfred.query = "buy $10 #660".to_string();
        let before = ticket_snapshot(&terminal);

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(ticket_snapshot(&terminal), before);
        assert_eq!(
            terminal.order_status,
            Some((
                "USD sizing is not supported for outcome markets; use contracts".to_string(),
                true
            ))
        );
    }

    #[test]
    fn alfred_trade_missing_signing_context_does_not_switch_or_mutate_ticket() {
        let mut terminal = alfred_trade_terminal();
        add_mid(&mut terminal, "ETH", 2_500.0);
        terminal.alfred.query = "buy 1 ETH".to_string();
        let before = ticket_snapshot(&terminal);

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(ticket_snapshot(&terminal), before);
        assert_eq!(
            terminal.order_status,
            Some(("Connect wallet and enter agent key first".to_string(), true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_trade_missing_signing_context_wins_over_missing_mid() {
        let mut terminal = alfred_trade_terminal();
        terminal.alfred.query = "buy 1 ETH".to_string();
        let before = ticket_snapshot(&terminal);

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(ticket_snapshot(&terminal), before);
        assert_eq!(
            terminal.order_status,
            Some(("Connect wallet and enter agent key first".to_string(), true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_trade_pending_request_does_not_switch_or_mutate_ticket() {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);
        add_mid(&mut terminal, "ETH", 2_500.0);
        terminal.pending_order_action = Some(PendingOrderAction::Sell);
        terminal.alfred.query = "buy 1 ETH".to_string();
        let before = ticket_snapshot(&terminal);

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(ticket_snapshot(&terminal), before);
        assert_eq!(
            terminal.order_status,
            Some((
                "Wait for pending trading requests to finish before placing an order".to_string(),
                true
            ))
        );
        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::Sell)
        );
    }

    #[test]
    fn alfred_trade_reconciliation_required_does_not_switch_or_mutate_ticket() {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);
        add_mid(&mut terminal, "ETH", 2_500.0);
        terminal.account_reconciliation_required = true;
        terminal.alfred.query = "buy 1 ETH".to_string();
        let before = ticket_snapshot(&terminal);

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(ticket_snapshot(&terminal), before);
        assert_eq!(
            terminal.order_status,
            Some((
                "Account refresh pending; wait for fresh account data before placing an order"
                    .to_string(),
                true
            ))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_market_trade_missing_mid_does_not_switch_or_mutate_ticket() {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);
        terminal.alfred.query = "buy 1 ETH".to_string();
        let before = ticket_snapshot(&terminal);

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(ticket_snapshot(&terminal), before);
        let (status, is_error) = order_status_or_panic(&terminal);
        assert!(is_error);
        assert!(status.starts_with("No mid price for ETH"));
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_market_trade_with_ready_context_still_updates_ticket_and_submits() {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);
        add_mid(&mut terminal, "ETH", 2_500.0);
        terminal.alfred.query = "buy 1 ETH".to_string();

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(terminal.active_symbol, "ETH");
        assert_eq!(terminal.order_kind, OrderKind::Market);
        assert_eq!(terminal.order_quantity, "1");
        assert!(!terminal.order_quantity_is_usd);
        assert_eq!(terminal.order_price, "");
        assert!(!terminal.presets_menu_expanded);
        assert!(!terminal.alfred.open);
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
    }

    #[test]
    fn alfred_spot_pair_trade_switches_to_and_submits_on_the_spot_market() {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);
        // A same-ticker perp exists: the explicit pair spelling must still
        // target the spot market, never the perp.
        terminal
            .exchange_symbols
            .push(symbol("HYPE", MarketType::Perp));
        terminal.exchange_symbols.push(spot_symbol("@107", "HYPE"));
        add_mid(&mut terminal, "@107", 25.0);
        terminal.alfred.query = "sell 1 hype/usdc".to_string();

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(terminal.active_symbol, "@107");
        assert_eq!(terminal.active_symbol_display, "HYPE/USDC");
        assert_eq!(terminal.order_kind, OrderKind::Market);
        assert_eq!(terminal.order_quantity, "1");
        assert!(!terminal.alfred.open);
        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::Sell)
        );
    }

    #[test]
    fn alfred_twap_preflight_preserves_existing_start_path() {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);

        assert!(terminal.alfred_trade_preflight_ready(
            "ETH",
            OrderKind::Twap,
            false,
            "1".to_string(),
            None,
            Some(true),
        ));
        assert_eq!(terminal.order_status, None);
    }

    #[test]
    fn alfred_close_pending_request_does_not_close_or_clear_close_menu() {
        let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
        add_mid(&mut terminal, "BTC", 50_000.0);
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

        assert!(terminal.alfred.open);
        assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
        assert_eq!(
            terminal.order_status,
            Some((
                "Wait for pending trading requests to finish before closing positions".to_string(),
                true
            ))
        );
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
    }

    #[test]
    fn alfred_close_missing_signing_context_does_not_close_or_clear_close_menu() {
        let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
        terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
        add_mid(&mut terminal, "BTC", 50_000.0);

        let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

        assert!(terminal.alfred.open);
        assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
        assert_eq!(
            terminal.order_status,
            Some(("Connect wallet and enter agent key first".to_string(), true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_close_missing_signing_context_wins_over_account_loading() {
        let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
        terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
        terminal.account_loading = true;
        add_mid(&mut terminal, "BTC", 50_000.0);

        let _task = terminal.alfred_close_position_preflight_task("BTC", 1.0);

        assert!(terminal.alfred.open);
        assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
        assert_eq!(
            terminal.order_status,
            Some(("Connect wallet and enter agent key first".to_string(), true))
        );
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_close_stale_account_does_not_close_or_clear_close_menu() {
        let mut terminal = alfred_close_terminal(1);
        add_mid(&mut terminal, "BTC", 50_000.0);

        let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

        assert!(terminal.alfred.open);
        assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
        let (status, is_error) = order_status_or_panic(&terminal);
        assert!(is_error);
        assert!(status.contains("Account data is stale"));
        assert!(status.contains("refresh before closing positions"));
        assert!(terminal.account_loading);
        assert!(terminal.pending_order_action.is_none());
    }

    #[test]
    fn alfred_close_missing_mid_does_not_close_or_clear_close_menu() {
        let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());

        let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

        assert!(terminal.alfred.open);
        assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
        let (status, is_error) = order_status_or_panic(&terminal);
        assert!(is_error);
        assert!(status.starts_with("No mid price for BTC"));
        assert!(terminal.pending_order_action.is_none());
    }
}
