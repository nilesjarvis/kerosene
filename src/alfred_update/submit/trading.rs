use crate::app_state::TradingTerminal;
use crate::helpers::parse_positive_number;
use crate::message::Message;
use crate::order_execution::{
    OrderOperation, OrderSurface, TicketOrderPlaceIntent, order_size_from_quantity_input,
    validate_surface_market_type,
};
use crate::signing::{ExchangeOrderKind, OrderKind};
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::Task;

// ---------------------------------------------------------------------------
// Alfred Trade Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn submit_alfred_trade(&mut self) -> Task<Message> {
        let Some(draft) = self.alfred_trade_draft(&self.alfred.query) else {
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

        let Some(symbol_key) = draft.symbol_key.as_deref() else {
            self.push_toast("Add a symbol".to_string(), true);
            return Task::none();
        };

        let submit_side = draft.side.map(|side| side.is_buy());
        if !self.alfred_trade_preflight_ready(
            symbol_key,
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
            self.switch_active_symbol_internal(symbol_key.to_string())
        };
        if self.active_symbol != symbol_key {
            let display = self.display_name_for_symbol(symbol_key);
            self.push_toast(format!("Cannot trade {display}"), true);
            return switch_task;
        }

        self.order_kind = draft.order_kind;
        self.order_quantity_is_usd = draft.quantity_is_usd;
        self.order_price = match draft.order_kind {
            OrderKind::Limit => draft.limit_price_input().unwrap_or_default(),
            OrderKind::Market | OrderKind::LimitIoc | OrderKind::Chase | OrderKind::Twap => {
                String::new()
            }
        };
        self.presets_menu_expanded = false;
        self.handle_order_quantity_changed(draft.quantity_input());
        self.persist_config();

        if let Some(side) = draft.side {
            return Task::batch([switch_task, self.execute_order(side.is_buy())]);
        }

        if draft.order_kind == OrderKind::Chase {
            let display = self.display_name_for_symbol(symbol_key);
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

    pub(super) fn alfred_trade_preflight_ready(
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
        else {
            self.order_status = Some((format!("Symbol '{symbol_key}' not found"), true));
            self.toast_order_status();
            return false;
        };
        if let Err(error) = self.validate_exchange_symbol_orderable(
            symbol,
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
}
