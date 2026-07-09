use crate::app_state::TradingTerminal;
use crate::config::OrderPreset;
use crate::helpers::{format_price_input, parse_number, positive_finite_value};
use crate::message::Message;
use crate::order_execution::{AdvancedOrderKind, OrderSurface, TicketOrderPlaceIntent};
use crate::signing::{ExchangeOrderKind, OrderKind};
use iced::Task;

impl TradingTerminal {
    pub(crate) fn handle_toggle_presets_menu(&mut self) {
        self.presets_menu_expanded = !self.presets_menu_expanded;
    }

    pub(crate) fn handle_toggle_preset_currency(&mut self) {
        self.preset_is_usd = !self.preset_is_usd;
        self.persist_config();
    }

    pub(crate) fn handle_toggle_preset_edit_mode(&mut self) {
        self.preset_edit_mode = !self.preset_edit_mode;
        self.preset_edit_idx = None;
    }

    pub(crate) fn handle_edit_preset_start(
        &mut self,
        kind: OrderKind,
        idx: usize,
        current_size_str: String,
    ) {
        self.preset_edit_idx = Some((kind, idx));
        self.preset_edit_buffer = current_size_str;
    }

    pub(crate) fn handle_edit_preset_changed(&mut self, new_text: String) {
        self.preset_edit_buffer = new_text;
    }

    pub(crate) fn handle_edit_preset_save(&mut self, kind: OrderKind, idx: usize) {
        if let Some(v) = parse_number(&self.preset_edit_buffer) {
            let prefix = if self.preset_is_usd { "$" } else { "" };
            let suffix = "";

            let update_preset = |presets: &mut Vec<OrderPreset>| {
                if let Some(preset) = presets.get_mut(idx) {
                    preset.size = v;
                    if let Some(pct) = preset.price_offset_pct {
                        preset.label = format!("-{pct}% {prefix}{v}{suffix}");
                    } else {
                        preset.label = format!("{prefix}{v}{suffix}");
                    }
                }
            };

            if self.preset_is_usd {
                match kind {
                    OrderKind::Market => update_preset(&mut self.order_presets.market_usd),
                    OrderKind::Limit | OrderKind::LimitIoc => {
                        update_preset(&mut self.order_presets.limit_usd)
                    }
                    OrderKind::Chase => update_preset(&mut self.order_presets.chase_usd),
                    OrderKind::Twap => {}
                }
            } else {
                match kind {
                    OrderKind::Market => update_preset(&mut self.order_presets.market_coin),
                    OrderKind::Limit | OrderKind::LimitIoc => {
                        update_preset(&mut self.order_presets.limit_coin)
                    }
                    OrderKind::Chase => update_preset(&mut self.order_presets.chase_coin),
                    OrderKind::Twap => {}
                }
            }
            self.persist_config();
        }
        self.preset_edit_idx = None;
    }

    pub(crate) fn handle_execute_preset(
        &mut self,
        kind: OrderKind,
        preset: OrderPreset,
        is_buy: bool,
    ) -> Task<Message> {
        if self.is_outcome_coin(&self.active_symbol) {
            return self.handle_execute_outcome_preset(kind, preset, is_buy);
        }
        if let Err(message) =
            self.validate_spot_quantity_denomination(&self.active_symbol, self.preset_is_usd)
        {
            self.order_status = Some((message, true));
            return Task::none();
        }

        let Some(mid) = self
            .resolve_mid_for_symbol(&self.active_symbol)
            .and_then(positive_finite_value)
        else {
            self.order_status =
                Some(("No mid price available for preset calculation".into(), true));
            return Task::none();
        };

        let Some(preset_size) = positive_finite_value(preset.size) else {
            self.order_status = Some(("Preset size must be a positive finite value".into(), true));
            return Task::none();
        };

        let qty = if self.preset_is_usd {
            preset_size / mid
        } else {
            preset_size
        };
        if !self.preset_order_preflight_ready(kind) {
            return Task::none();
        }

        let quantity_input = format!("{qty:.6}");
        let order_price = if kind == OrderKind::Limit {
            let target_price = if let Some(pct) = preset.price_offset_pct {
                let offset = pct / 100.0;
                if is_buy {
                    mid * (1.0 - offset)
                } else {
                    mid * (1.0 + offset)
                }
            } else {
                mid
            };
            // Low-priced spot markets carry up to 8 decimal places; a fixed
            // 4-decimal render would corrupt (or zero out) the preset offset.
            format_price_input(target_price)
        } else {
            String::new()
        };
        if matches!(kind, OrderKind::Market | OrderKind::Limit) {
            let exchange_order_kind = match ExchangeOrderKind::try_from(kind) {
                Ok(kind) => kind,
                Err(message) => {
                    self.order_status = Some((message.into(), true));
                    return Task::none();
                }
            };
            let intent = Self::ticket_order_place_intent(TicketOrderPlaceIntent {
                surface: OrderSurface::Preset,
                symbol_key: self.active_symbol.clone(),
                is_buy,
                order_kind: exchange_order_kind,
                price_input: order_price.clone(),
                quantity_input: quantity_input.clone(),
                quantity_is_usd: false,
                reduce_only: self.order_reduce_only,
            });
            if let Err(message) = self.prepare_place_order(intent) {
                self.order_status = Some((message, true));
                return Task::none();
            }
        }

        self.order_kind = kind;
        self.order_quantity_provenance = None;
        self.order_quantity = quantity_input;
        self.order_quantity_is_usd = false;
        self.order_percentage = 0.0;

        if kind == OrderKind::Limit || kind == OrderKind::Market {
            if kind == OrderKind::Limit {
                self.order_price = order_price;
            } else if kind == OrderKind::Market {
                self.order_price.clear();
            }

            self.presets_menu_expanded = false;
            self.execute_order_with_surface(is_buy, OrderSurface::Preset)
        } else if kind == OrderKind::Chase {
            self.presets_menu_expanded = false;
            self.start_chase(is_buy)
        } else if kind == OrderKind::Twap {
            self.presets_menu_expanded = false;
            self.start_twap(is_buy)
        } else {
            Task::none()
        }
    }

    pub(crate) fn preset_order_preflight_ready(&mut self, kind: OrderKind) -> bool {
        match kind {
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc => {
                if self.reject_if_pending_trading_request("placing an order") {
                    return false;
                }
                if self
                    .reject_if_account_reconciliation_required("placing an order", "account data")
                {
                    return false;
                }
                self.checked_order_signing_account().is_some()
            }
            OrderKind::Chase => self.advanced_order_start_preflight_ready(AdvancedOrderKind::Chase),
            OrderKind::Twap => self.advanced_order_start_preflight_ready(AdvancedOrderKind::Twap),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};
    use crate::app_state::sensitive_string;
    use crate::config::{AccountProfile, OrderPreset};
    use crate::order_execution::PendingOrderAction;
    use crate::order_pending_indicators::PendingOrderIndicatorKind;
    use crate::signing::OrderKind;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

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

    fn preset_ready_terminal() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        connect_test_account(&mut terminal);
        terminal.active_symbol = "BTC".to_string();
        terminal.active_symbol_display = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 50_000.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        terminal.preset_is_usd = false;
        terminal.order_quantity_is_usd = true;
        terminal.order_percentage = 25.0;
        terminal.pending_order_action = None;
        terminal.twap_form.duration_minutes = "5".to_string();
        terminal.twap_form.slices = "2".to_string();
        terminal.twap_form.min_price = "90".to_string();
        terminal.twap_form.max_price = "110".to_string();
        terminal.twap_form.randomize = false;
        terminal
    }

    fn base_preset(size: f64) -> OrderPreset {
        OrderPreset {
            label: size.to_string(),
            size,
            price_offset_pct: None,
        }
    }

    fn seed_existing_ticket(terminal: &mut TradingTerminal) {
        terminal.order_kind = OrderKind::Market;
        terminal.order_quantity = "old-size".to_string();
        terminal.order_price = "0.42".to_string();
        terminal.order_quantity_is_usd = true;
        terminal.order_percentage = 25.0;
        terminal.presets_menu_expanded = true;
    }

    fn assert_existing_ticket_unchanged(terminal: &TradingTerminal) {
        assert_eq!(terminal.order_kind, OrderKind::Market);
        assert_eq!(terminal.order_quantity, "old-size");
        assert_eq!(terminal.order_price, "0.42");
        assert!(terminal.order_quantity_is_usd);
        assert_eq!(terminal.order_percentage, 25.0);
    }

    #[test]
    fn preset_without_mid_does_not_mutate_order_ticket() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(OrderKind::Limit, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some((
                "No mid price available for preset calculation".to_string(),
                true
            ))
        );
    }

    #[test]
    fn invalid_preset_size_does_not_mutate_order_ticket() {
        let mut terminal = preset_ready_terminal();
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(OrderKind::Limit, base_preset(0.0), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some((
                "Preset size must be a positive finite value".to_string(),
                true
            ))
        );
    }

    #[test]
    fn unsupported_outcome_preset_does_not_mutate_order_ticket_from_top_level() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "#650".to_string();
        terminal.exchange_symbols = vec![symbol("#650", MarketType::Outcome)];
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(OrderKind::Chase, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(!terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some(("Outcome automation is not supported yet".to_string(), true))
        );
    }

    #[test]
    fn preset_missing_signing_context_does_not_mutate_order_ticket() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.all_mids.insert("BTC".to_string(), 50_000.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(OrderKind::Market, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some(("Connect wallet and enter agent key first".to_string(), true))
        );
    }

    #[test]
    fn preset_active_wallet_mismatch_does_not_mutate_order_ticket() {
        let mut terminal = preset_ready_terminal();
        seed_existing_ticket(&mut terminal);
        terminal.wallet_address_input = "0xdef0000000000000000000000000000000000000".to_string();
        terminal.accounts[0].wallet_address =
            "0xdef0000000000000000000000000000000000000".to_string();

        let _task = terminal.handle_execute_preset(OrderKind::Market, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some((
                "Connected wallet no longer matches the active account; reconnect before trading"
                    .to_string(),
                true,
            ))
        );
    }

    #[test]
    fn preset_pending_request_does_not_mutate_order_ticket() {
        let mut terminal = preset_ready_terminal();
        seed_existing_ticket(&mut terminal);
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.handle_execute_preset(OrderKind::Limit, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some((
                "Wait for pending trading requests to finish before placing an order".to_string(),
                true,
            ))
        );
    }

    #[test]
    fn preset_reconciliation_required_does_not_mutate_order_ticket() {
        let mut terminal = preset_ready_terminal();
        seed_existing_ticket(&mut terminal);
        terminal.account_reconciliation_required = true;

        let _task = terminal.handle_execute_preset(OrderKind::Market, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert_eq!(
            terminal.order_status,
            Some((
                "Account refresh pending; wait for fresh account data before placing an order"
                    .to_string(),
                true,
            ))
        );
    }

    #[test]
    fn chase_preset_pending_request_does_not_mutate_order_ticket() {
        let mut terminal = preset_ready_terminal();
        seed_existing_ticket(&mut terminal);
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.handle_execute_preset(OrderKind::Chase, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert!(terminal.chase_orders.is_empty());
        assert_eq!(
            terminal.order_status,
            Some((
                "Wait for pending trading requests to finish before starting a chase".to_string(),
                true,
            ))
        );
    }

    #[test]
    fn limit_preset_without_offset_uses_current_mid_instead_of_stale_ticket_price() {
        let mut terminal = preset_ready_terminal();
        terminal.order_price = "1".to_string();

        let _task = terminal.handle_execute_preset(OrderKind::Limit, base_preset(2.5), true);

        assert_eq!(terminal.order_kind, OrderKind::Limit);
        assert_eq!(terminal.order_quantity, "2.500000");
        assert_eq!(terminal.order_price, "50000.0000");
        assert!(!terminal.order_quantity_is_usd);
        assert!(!terminal.presets_menu_expanded);
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
        let indicator = terminal
            .pending_order_indicators
            .values()
            .next()
            .expect("preset limit submit should add a pending indicator");
        assert_eq!(indicator.kind, PendingOrderIndicatorKind::Placing);
        assert_eq!(indicator.symbol, "BTC");
        assert!(indicator.is_buy);
        assert_eq!(indicator.size, "2.5");
        assert_eq!(indicator.price, "50000");
    }

    #[test]
    fn limit_preset_offset_keeps_precision_on_low_priced_spot_markets() {
        let mut terminal = preset_ready_terminal();
        let mut spot = symbol("@107", MarketType::Spot);
        spot.ticker = "MEME".to_string();
        spot.display_name = Some("MEME/USDC".to_string());
        spot.sz_decimals = 0;
        spot.asset_index = 10_107;
        terminal.active_symbol = "@107".to_string();
        terminal.active_symbol_display = "MEME/USDC".to_string();
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@107".to_string(), 0.00035);
        terminal
            .all_mids_updated_at_ms
            .insert("@107".to_string(), TradingTerminal::now_ms());

        let _task = terminal.handle_execute_preset(
            OrderKind::Limit,
            OrderPreset {
                label: "-1% 100000".to_string(),
                size: 100_000.0,
                price_offset_pct: Some(1.0),
            },
            true,
        );

        // A fixed 4-decimal format would submit 0.0003 (-14.3% instead of
        // the requested -1%) on a market that carries 8 decimal places.
        assert_eq!(terminal.order_price, "0.0003465");
        assert_eq!(terminal.order_kind, OrderKind::Limit);
        assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
    }

    #[test]
    fn preset_prepare_failure_does_not_mutate_order_ticket_or_close_menu() {
        let mut terminal = preset_ready_terminal();
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(
            OrderKind::Limit,
            OrderPreset {
                label: "-99% 2.5".to_string(),
                size: 2.5,
                price_offset_pct: Some(99.0),
            },
            true,
        );

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert!(terminal.pending_order_action.is_none());
        let status = terminal
            .order_status
            .as_ref()
            .expect("prepare failure should set status");
        assert!(status.1);
        assert!(status.0.starts_with("Order price"));
    }

    #[test]
    fn usd_preset_writes_coin_quantity_and_clears_ticket_usd_denomination() {
        let mut terminal = preset_ready_terminal();
        terminal.preset_is_usd = true;
        terminal.order_quantity_is_usd = true;
        terminal.order_percentage = 25.0;

        let _task = terminal.handle_execute_preset(
            OrderKind::Market,
            OrderPreset {
                label: "$100".to_string(),
                size: 100.0,
                price_offset_pct: None,
            },
            true,
        );

        assert_eq!(terminal.order_quantity, "0.002000");
        assert!(!terminal.order_quantity_is_usd);
        assert_eq!(terminal.order_percentage, 0.0);
    }

    #[test]
    fn usd_preset_rejects_non_usd_quoted_spot_without_mutating_ticket() {
        let mut terminal = preset_ready_terminal();
        let mut spot = symbol("@55", MarketType::Spot);
        spot.ticker = "UETH".to_string();
        spot.display_name = Some("UETH/UBTC".to_string());
        spot.asset_index = 10_055;
        terminal.active_symbol = "@55".to_string();
        terminal.active_symbol_display = "UETH/UBTC".to_string();
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@55".to_string(), 0.05);
        terminal
            .all_mids_updated_at_ms
            .insert("@55".to_string(), TradingTerminal::now_ms());
        terminal.preset_is_usd = true;
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(
            OrderKind::Market,
            OrderPreset {
                label: "$100".to_string(),
                size: 100.0,
                price_offset_pct: None,
            },
            true,
        );

        assert_existing_ticket_unchanged(&terminal);
        assert!(terminal.presets_menu_expanded);
        assert!(terminal.pending_order_action.is_none());
        assert_eq!(
            terminal.order_status,
            Some((
                "Spot trading is unavailable for UETH/UBTC because quote-token USD valuation and accounting are not verified".to_string(),
                true,
            ))
        );
    }

    #[test]
    fn coin_preset_cannot_bypass_non_usd_quote_gate() {
        let mut terminal = preset_ready_terminal();
        let mut spot = symbol("@55", MarketType::Spot);
        spot.ticker = "UETH".to_string();
        spot.display_name = Some("UETH/UBTC".to_string());
        spot.asset_index = 10_055;
        terminal.active_symbol = "@55".to_string();
        terminal.active_symbol_display = "UETH/UBTC".to_string();
        terminal.exchange_symbols = vec![spot];
        terminal.all_mids.insert("@55".to_string(), 0.05);
        terminal
            .all_mids_updated_at_ms
            .insert("@55".to_string(), TradingTerminal::now_ms());
        terminal.preset_is_usd = false;
        seed_existing_ticket(&mut terminal);

        let _task = terminal.handle_execute_preset(OrderKind::Market, base_preset(2.5), true);

        assert_existing_ticket_unchanged(&terminal);
        assert_eq!(terminal.pending_order_action, None);
        assert!(
            terminal
                .order_status
                .as_ref()
                .is_some_and(|(message, is_error)| {
                    *is_error
                        && message
                            .contains("quote-token USD valuation and accounting are not verified")
                })
        );
    }

    #[test]
    fn chase_preset_starts_for_click_context_immediately() {
        let mut terminal = preset_ready_terminal();

        let _task = terminal.handle_execute_preset(OrderKind::Chase, base_preset(2.5), true);
        terminal.active_symbol = "ETH".to_string();

        let chase = terminal
            .selected_chase()
            .expect("chase preset should create a chase synchronously");
        assert_eq!(terminal.chase_orders.len(), 1);
        assert_eq!(chase.coin, "BTC");
        assert_eq!(chase.target_size, 2.5);
        assert_eq!(chase.account_address, TEST_ACCOUNT);
        assert_eq!(
            terminal.pending_order_action,
            Some(PendingOrderAction::ChaseBuy)
        );
    }

    #[test]
    fn twap_preset_starts_for_click_context_immediately() {
        let mut terminal = preset_ready_terminal();

        let _task = terminal.handle_execute_preset(OrderKind::Twap, base_preset(2.5), false);
        terminal.active_symbol = "ETH".to_string();

        let twap_id = terminal
            .selected_twap_id
            .expect("twap preset should create a TWAP synchronously");
        let twap = terminal
            .twap_orders
            .get(&twap_id)
            .expect("selected TWAP should exist");
        assert_eq!(terminal.twap_orders.len(), 1);
        assert_eq!(twap.coin, "BTC");
        assert_eq!(twap.target_size, 2.5);
        assert_eq!(twap.account_address, TEST_ACCOUNT);
        assert!(!twap.is_buy);
        assert_eq!(terminal.pending_order_action, None);
    }
}
