use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_number, positive_finite_value};
use crate::message::Message;
use crate::order_execution::{
    AdvancedOrderKind, OrderOperation, OrderSurface, TwapOrderStartSnapshot,
    validate_surface_market_type,
};
use crate::signing::float_to_wire;
use crate::twap_state::{
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_DUPLICATE_START_WINDOW, TwapOrder, TwapOrderInit,
    twap_min_quantized_child_notional, twap_required_slice_rate, twap_target_size_from_quantity,
};
use iced::Task;
use std::time::Instant;

mod price_range;
mod validation;

use super::super::sizing::order_size_from_quantity_input;
use validation::{parse_twap_start_schedule, validate_twap_schedule_capacity};

impl TradingTerminal {
    fn active_twap_slice_rate(&self, now: Instant) -> f64 {
        self.twap_orders
            .values()
            .filter(|twap| !twap.status.is_terminal() && !twap.stop_requested)
            .filter_map(|twap| {
                let remaining_slices = twap.slice_count.saturating_sub(twap.slices_attempted);
                twap_required_slice_rate(
                    twap.ends_at.saturating_duration_since(now),
                    remaining_slices,
                )
            })
            .sum()
    }

    pub(crate) fn next_twap_id(&mut self) -> u64 {
        let id = self.next_twap_id;
        self.next_twap_id = self.next_twap_id.checked_add(1).unwrap_or(1);
        id
    }

    pub(crate) fn handle_twap_duration_changed(&mut self, value: String) {
        self.twap_form.duration_minutes = value;
    }

    pub(crate) fn handle_twap_slices_changed(&mut self, value: String) {
        self.twap_form.slices = value;
    }

    pub(crate) fn handle_twap_min_price_changed(&mut self, value: String) {
        self.twap_form.min_price = value;
    }

    pub(crate) fn handle_twap_max_price_changed(&mut self, value: String) {
        self.twap_form.max_price = value;
    }

    pub(crate) fn handle_twap_randomize_toggled(&mut self, value: bool) {
        self.twap_form.randomize = value;
    }

    pub(crate) fn start_twap_from_snapshot(
        &mut self,
        is_buy: bool,
        snapshot: TwapOrderStartSnapshot,
    ) -> Task<Message> {
        if self.reject_stale_twap_order_start_snapshot(&snapshot) {
            return Task::none();
        }

        self.start_twap(is_buy)
    }

    pub(crate) fn start_twap(&mut self, is_buy: bool) -> Task<Message> {
        let Some(start_context) = self.advanced_order_start_context(AdvancedOrderKind::Twap) else {
            return Task::none();
        };
        if let Err(message) = self
            .validate_spot_quantity_denomination(&self.active_symbol, self.order_quantity_is_usd)
        {
            self.order_status = Some((message, true));
            return Task::none();
        }
        if let Err(message) = self.validate_spot_automation_quote(&self.active_symbol, "TWAP") {
            self.order_status = Some((message, true));
            return Task::none();
        }
        if let Some(task) = self.stale_percentage_order_quantity_task("starting a TWAP", is_buy) {
            return task;
        }

        let now = Instant::now();
        // start_twap completes synchronously, so the pending-order flag never
        // covers it; this window is what absorbs a double click that would
        // otherwise start two full-size TWAPs.
        if self.twap_orders.values().any(|twap| {
            !twap.status.is_terminal()
                && !twap.stop_requested
                && twap.coin == self.active_symbol
                && twap.is_buy == is_buy
                && now.saturating_duration_since(twap.started_at) < TWAP_DUPLICATE_START_WINDOW
        }) {
            self.order_status = Some((
                "A TWAP for this symbol and side just started; wait a moment to start another"
                    .into(),
                true,
            ));
            return Task::none();
        }

        let Some(sym) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == self.active_symbol)
            .cloned()
        else {
            self.order_status = Some((
                format!(
                    "Symbol '{}' not found in exchange metadata",
                    self.active_symbol
                ),
                true,
            ));
            return Task::none();
        };
        if let Err(error) = self.validate_exchange_symbol_orderable(
            &sym,
            OrderSurface::Twap.orderability_context_label(),
        ) {
            self.order_status = Some((error, true));
            return Task::none();
        }
        if let Err(error) =
            validate_surface_market_type(OrderSurface::Twap, OrderOperation::Place, sym.market_type)
        {
            self.order_status = Some((error.status_text(), true));
            return Task::none();
        }

        let schedule = match parse_twap_start_schedule(
            &self.twap_form.duration_minutes,
            &self.twap_form.slices,
        ) {
            Ok(schedule) => schedule,
            Err(message) => {
                self.order_status = Some((message, true));
                return Task::none();
            }
        };
        let active_slice_rate = self.active_twap_slice_rate(now);
        if let Err(message) = validate_twap_schedule_capacity(
            active_slice_rate,
            schedule.duration,
            schedule.slice_count,
        ) {
            self.order_status = Some((message, true));
            return Task::none();
        }

        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_spot = sym.market_type == MarketType::Spot;
        let reduce_only = if is_spot {
            false
        } else {
            self.order_reduce_only
        };

        let Some((min_price, max_price)) = self.effective_twap_price_range(sz_decimals, is_spot)
        else {
            self.order_status = Some((
                "Invalid TWAP price range: set finite min and max prices with min < max".into(),
                true,
            ));
            return Task::none();
        };

        let raw_qty = match parse_number(&self.order_quantity).and_then(positive_finite_value) {
            Some(value) => value,
            _ => {
                self.order_status = Some(("Invalid quantity".into(), true));
                return Task::none();
            }
        };
        let reference_price = if self.order_quantity_is_usd {
            match self
                .resolve_mid_for_symbol(&self.active_symbol)
                .and_then(positive_finite_value)
            {
                Some(price) => Some(price),
                None => {
                    self.order_status = Some((
                        format!(
                            concat!(
                                "Cannot start USD TWAP: no fresh mid price for {}. ",
                                "Wait for market data or enter size in coin units."
                            ),
                            self.active_symbol_display
                        ),
                        true,
                    ));
                    return Task::none();
                }
            }
        } else {
            None
        };
        let exact_spot_percentage = is_spot
            .then(|| self.ticket_spot_percentage_balance_for_side(is_buy))
            .flatten();
        let target_size = if let Some((balance, percentage)) = exact_spot_percentage {
            let available = balance * (percentage as f64 / 100.0);
            order_size_from_quantity_input(
                available,
                // A buy TWAP may execute anywhere in its configured range;
                // size against the worst permitted price.
                max_price,
                is_buy,
                sz_decimals,
            )
        } else {
            twap_target_size_from_quantity(raw_qty, reference_price, self.order_quantity_is_usd)
        };
        let Some(target_size) = target_size else {
            self.order_status = Some(("Invalid TWAP size".into(), true));
            return Task::none();
        };
        let min_child_notional = twap_min_quantized_child_notional(
            target_size,
            schedule.slice_count,
            min_price,
            self.twap_form.randomize,
            sz_decimals,
        )
        .unwrap_or(0.0);
        if min_child_notional < MIN_EXCHANGE_ORDER_NOTIONAL_USD {
            self.order_status = Some((
                format!(
                    concat!(
                        "Cannot start TWAP: smallest planned slice is ${:.2}; ",
                        "Hyperliquid requires at least ${:.0} per child order. ",
                        "Increase size or reduce slices."
                    ),
                    min_child_notional, MIN_EXCHANGE_ORDER_NOTIONAL_USD
                ),
                true,
            ));
            return Task::none();
        }

        let twap_id = self.next_twap_id();
        let side = if is_buy { "BUY" } else { "SELL" };
        let twap = TwapOrder::new(TwapOrderInit {
            id: twap_id,
            coin: self.active_symbol.clone(),
            display_coin: self.active_symbol_display.clone(),
            account_address: start_context.account_address,
            agent_key: start_context.agent_key,
            is_buy,
            target_size,
            asset,
            sz_decimals,
            is_spot,
            reduce_only,
            min_price,
            max_price,
            randomize: self.twap_form.randomize,
            duration: schedule.duration,
            slice_count: schedule.slice_count,
            now,
            started_at_ms: Self::now_ms(),
        });
        self.twap_orders.insert(twap_id, twap);
        if is_spot {
            self.record_twap_spot_symbol_identity(twap_id, &sym);
        }
        self.selected_twap_id = Some(twap_id);
        self.order_status = Some((
            format!(
                "TWAP {side} {} {} over {} slices: waiting for market data",
                float_to_wire(target_size),
                self.active_symbol_display,
                schedule.slice_count
            ),
            false,
        ));
        Task::none()
    }
}

#[cfg(test)]
mod tests;
