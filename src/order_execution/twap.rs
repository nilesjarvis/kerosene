use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use crate::signing::{OrderKind, cancel_order, float_to_wire, place_order, round_price};
use crate::twap_state::{
    ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL, MAX_ACTIVE_ADVANCED_ORDERS,
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_BOOK_STALE_AFTER, TwapBookSnapshot, TwapChildOrder,
    TwapChildStatus, TwapEventKind, TwapOrder, TwapPendingOp, TwapPendingSlice, TwapStatus,
    parse_twap_duration_minutes, parse_twap_slice_count, quantize_twap_slice_size,
    twap_aggregate_schedule_has_capacity, twap_aggregate_slice_rate, twap_limit_price_for_slice,
    twap_min_quantized_child_notional, twap_order_notional_meets_minimum, twap_required_slice_rate,
    twap_response_fill_summary, twap_target_size_from_quantity, validate_twap_interval,
};
use iced::{Size, Task, window};
use std::time::Instant;

// ---------------------------------------------------------------------------
// TWAP Advanced Orders
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TwapAccountRefresh {
    None,
    OnTerminal,
    Immediate,
}

impl TwapAccountRefresh {
    fn should_refresh(self, twap_is_terminal: bool) -> bool {
        match self {
            Self::None => false,
            Self::OnTerminal => twap_is_terminal,
            Self::Immediate => true,
        }
    }
}

fn twap_ioc_limit_price(
    raw_price: f64,
    is_buy: bool,
    sz_decimals: u32,
    is_spot: bool,
    min_price: f64,
    max_price: f64,
) -> Option<f64> {
    if !raw_price.is_finite()
        || raw_price <= 0.0
        || !min_price.is_finite()
        || !max_price.is_finite()
        || min_price <= 0.0
        || max_price < min_price
        || raw_price < min_price
        || raw_price > max_price
    {
        return None;
    }

    let rounded = round_price(raw_price, sz_decimals, is_spot);
    if !rounded.is_finite() || rounded <= 0.0 {
        return None;
    }

    let price = if is_buy {
        if rounded < raw_price || rounded > max_price {
            raw_price
        } else {
            rounded
        }
    } else if rounded > raw_price || rounded < min_price {
        raw_price
    } else {
        rounded
    };

    (price.is_finite() && price > 0.0 && price >= min_price && price <= max_price).then_some(price)
}

impl TradingTerminal {
    pub(crate) fn active_advanced_order_count(&self) -> usize {
        self.chase_orders
            .values()
            .filter(|chase| !chase.stop_requested)
            .count()
            + self
                .twap_orders
                .values()
                .filter(|twap| !twap.status.is_terminal() && !twap.stop_requested)
                .count()
    }

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

    pub(crate) fn start_twap(&mut self, is_buy: bool) -> Task<Message> {
        if self.active_advanced_order_count() >= MAX_ACTIVE_ADVANCED_ORDERS {
            self.order_status = Some((
                format!(
                    "Cannot start TWAP: maximum of {MAX_ACTIVE_ADVANCED_ORDERS} active advanced orders reached"
                ),
                true,
            ));
            return Task::none();
        }
        if self.pending_order_action.is_some() {
            self.order_status = Some(("Wait for the pending order action to finish".into(), true));
            return Task::none();
        }

        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        let Some(account_address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        };
        if self.is_ticker_muted(&self.active_symbol) {
            self.order_status = Some(("Active ticker is muted in Settings > Risk".into(), true));
            return Task::none();
        }

        let Some(sym) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == self.active_symbol)
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
        if sym.market_type == MarketType::Outcome {
            self.outcome_read_only_status("TWAP trading");
            return Task::none();
        }

        let duration = match parse_twap_duration_minutes(&self.twap_form.duration_minutes) {
            Some(duration) => duration,
            None => {
                self.order_status = Some((
                    "Invalid TWAP duration: use 1 minute to 24 hours".into(),
                    true,
                ));
                return Task::none();
            }
        };
        let slice_count = match parse_twap_slice_count(&self.twap_form.slices) {
            Some(slice_count) => slice_count,
            None => {
                self.order_status = Some((
                    format!(
                        "Invalid TWAP slices: use 1 to {}",
                        crate::twap_state::TWAP_MAX_SLICES
                    ),
                    true,
                ));
                return Task::none();
            }
        };
        if !validate_twap_interval(duration, slice_count) {
            self.order_status = Some((
                "TWAP interval is too short: use at least 5 seconds per slice".into(),
                true,
            ));
            return Task::none();
        }
        let now = Instant::now();
        let active_slice_rate = self.active_twap_slice_rate(now);
        if !twap_aggregate_schedule_has_capacity(active_slice_rate, duration, slice_count) {
            let combined_rate =
                twap_aggregate_slice_rate(active_slice_rate, duration, slice_count).unwrap_or(0.0);
            self.order_status = Some((
                format!(
                    "Cannot start TWAP: active TWAP schedule is too dense ({combined_rate:.2} slices/sec). Increase duration, reduce slices, or wait for another TWAP to finish."
                ),
                true,
            ));
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

        let raw_qty = match self.order_quantity.trim().parse::<f64>() {
            Ok(value) if value.is_finite() && value > 0.0 => value,
            _ => {
                self.order_status = Some(("Invalid quantity".into(), true));
                return Task::none();
            }
        };
        let reference_price = if self.order_quantity_is_usd {
            match self
                .resolve_mid_for_symbol(&self.active_symbol)
                .filter(|price| price.is_finite() && *price > 0.0)
            {
                Some(price) => Some(price),
                None => {
                    self.order_status = Some((
                        format!(
                            "Cannot start USD TWAP: no fresh mid price for {}. Wait for market data or enter size in coin units.",
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
        let Some(target_size) =
            twap_target_size_from_quantity(raw_qty, reference_price, self.order_quantity_is_usd)
        else {
            self.order_status = Some(("Invalid TWAP size".into(), true));
            return Task::none();
        };
        if !target_size.is_finite() || target_size <= 0.0 {
            self.order_status = Some(("Invalid TWAP size".into(), true));
            return Task::none();
        }
        let min_child_notional = twap_min_quantized_child_notional(
            target_size,
            slice_count,
            min_price,
            self.twap_form.randomize,
            sz_decimals,
        )
        .unwrap_or(0.0);
        if min_child_notional < MIN_EXCHANGE_ORDER_NOTIONAL_USD {
            self.order_status = Some((
                format!(
                    "Cannot start TWAP: smallest planned slice is ${min_child_notional:.2}; Hyperliquid requires at least ${MIN_EXCHANGE_ORDER_NOTIONAL_USD:.0} per child order. Increase size or reduce slices."
                ),
                true,
            ));
            return Task::none();
        }

        let twap_id = self.next_twap_id();
        let side = if is_buy { "BUY" } else { "SELL" };
        let twap = TwapOrder::new(
            twap_id,
            self.active_symbol.clone(),
            self.active_symbol_display.clone(),
            account_address,
            key.into(),
            is_buy,
            target_size,
            asset,
            sz_decimals,
            is_spot,
            reduce_only,
            min_price,
            max_price,
            self.twap_form.randomize,
            duration,
            slice_count,
            now,
            Self::now_ms(),
        );
        self.twap_orders.insert(twap_id, twap);
        self.selected_twap_id = Some(twap_id);
        self.order_status = Some((
            format!(
                "TWAP {side} {} {} over {} slices: waiting for market data",
                float_to_wire(target_size),
                self.active_symbol_display,
                slice_count
            ),
            false,
        ));
        Task::none()
    }

    pub(crate) fn stop_twap(&mut self, twap_id: u64) -> Task<Message> {
        self.stop_twap_with_reason(twap_id, "TWAP stopped", false)
    }

    pub(crate) fn stop_all_twaps(&mut self) -> Task<Message> {
        let ids: Vec<_> = self
            .twap_orders
            .iter()
            .filter_map(|(id, twap)| {
                (!twap.status.is_terminal() && !twap.stop_requested).then_some(*id)
            })
            .collect();
        for id in ids {
            let _ = self.stop_twap_with_reason(id, "TWAP stopped", false);
        }
        Task::none()
    }

    pub(crate) fn stop_twap_with_reason(
        &mut self,
        twap_id: u64,
        reason: impl Into<String>,
        is_error: bool,
    ) -> Task<Message> {
        let reason = reason.into();
        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return Task::none();
        };
        twap.stop_requested = true;
        twap.stop_reason = Some((reason.clone(), is_error));
        if twap.pending_op.is_some() {
            twap.status = TwapStatus::Stopping;
            self.order_status = Some((format!("{reason}: waiting for in-flight slice"), is_error));
        } else {
            twap.status = TwapStatus::Stopped;
            twap.push_event(TwapEventKind::Stopped, reason.clone(), is_error);
            self.order_status = Some((reason, is_error));
        }
        self.archive_twap_if_terminal(twap_id);
        Task::none()
    }

    pub(crate) fn handle_twap_book_update(
        &mut self,
        twap_id: u64,
        coin: String,
        book: crate::api::OrderBook,
    ) -> Task<Message> {
        if self.is_ticker_muted(&coin) {
            return Task::none();
        }
        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return Task::none();
        };
        if twap.coin != coin || twap.status.is_terminal() || twap.stop_requested {
            return Task::none();
        }
        twap.latest_book = Some(TwapBookSnapshot {
            book,
            updated_at: Instant::now(),
        });
        if twap.status == TwapStatus::WaitingForMarket {
            twap.status = TwapStatus::Running;
        }
        Task::none()
    }

    pub(crate) fn handle_twap_tick(&mut self) -> Task<Message> {
        let now = Instant::now();
        let Some(twap_id) = self
            .twap_orders
            .iter()
            .filter(|(_, twap)| twap.can_schedule() && twap.next_slice_due <= now)
            .map(|(id, _)| *id)
            .next()
        else {
            return Task::none();
        };
        self.execute_due_twap_slice(twap_id, now)
    }

    pub(crate) fn handle_twap_slice_result(
        &mut self,
        twap_id: u64,
        result: Result<crate::signing::ExchangeResponse, String>,
    ) -> Task<Message> {
        let mut refresh_policy = twap_place_result_refresh_policy(&result);
        let now = Instant::now();
        let pending = self
            .twap_orders
            .get(&twap_id)
            .and_then(|twap| match twap.pending_op {
                Some(TwapPendingOp::Place(slice)) => Some(slice),
                _ => None,
            });
        let Some(pending) = pending else {
            return self.refresh_after_twap_result(refresh_policy, twap_id);
        };

        let mut status_update = None;
        let mut cancel_unexpected = None;
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            twap.pending_op = None;
            match result {
                Ok(response) => {
                    let summary_text = response.summary();
                    let fill_summary = twap_response_fill_summary(&response);
                    let oid = fill_summary.oid.or_else(|| response.order_oid());

                    if let Some(child) = twap
                        .child_orders
                        .iter_mut()
                        .find(|child| child.index == pending.index)
                    {
                        child.oid = oid;
                        child.exchange_summary = summary_text.clone();
                        child.filled_size = child.filled_size.max(fill_summary.filled_size);
                        child.avg_price = fill_summary.avg_price.or(child.avg_price);
                    }

                    if response.is_ioc_no_match() {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::NoFill;
                        }
                        twap.push_event(
                            TwapEventKind::Placed,
                            format!(
                                "Slice {} did not fill: book moved before the IOC could match",
                                pending.index
                            ),
                            false,
                        );
                        status_update = Some((
                            format!("TWAP slice {} did not fill; continuing", pending.index),
                            false,
                        ));
                    } else if response.is_error() {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::Rejected;
                        }
                        twap.push_event(
                            TwapEventKind::Rejected,
                            format!("Slice {} rejected: {summary_text}", pending.index),
                            true,
                        );
                        status_update = Some((
                            format!("TWAP slice {} rejected: {summary_text}", pending.index),
                            true,
                        ));
                    } else if fill_summary.filled_size > 0.0 {
                        let filled_size = fill_summary.filled_size;
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::Filled;
                            child.filled_size = child.filled_size.max(filled_size);
                        }
                        twap.mark_filled(filled_size);
                        twap.push_event(
                            TwapEventKind::Filled,
                            format!(
                                "Slice {} filled {} @ {}",
                                pending.index,
                                float_to_wire(filled_size),
                                fill_summary
                                    .avg_price
                                    .map(format_price)
                                    .unwrap_or_else(|| format_price(pending.limit_price))
                            ),
                            false,
                        );
                        status_update = Some((
                            format!(
                                "TWAP slice {} filled {} {}",
                                pending.index,
                                float_to_wire(filled_size),
                                twap.display_coin
                            ),
                            false,
                        ));
                    } else if response.is_fully_filled() {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::StatusUnknown;
                        }
                        twap.status = TwapStatus::Error;
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                "Slice {} reported filled but fill size was unavailable; refreshing account data",
                                pending.index
                            ),
                            true,
                        );
                        status_update = Some((
                            format!(
                                "TWAP slice {} fill size unknown; refreshing account data",
                                pending.index
                            ),
                            true,
                        ));
                        refresh_policy = TwapAccountRefresh::Immediate;
                    } else if let Some(oid) = oid {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::UnexpectedResting;
                        }
                        twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting { oid });
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                "Slice {} unexpectedly rested as oid {oid}; cancelling",
                                pending.index
                            ),
                            true,
                        );
                        cancel_unexpected =
                            Some((twap.agent_key.trim().to_string(), twap.asset, oid));
                        status_update = Some((
                            format!(
                                "TWAP slice {} unexpectedly rested; cancelling",
                                pending.index
                            ),
                            true,
                        ));
                    } else if response.is_ambiguous_order_result() {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::StatusUnknown;
                        }
                        twap.status = TwapStatus::Error;
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                "Slice {} returned ambiguous order status: {summary_text}; refreshing account data",
                                pending.index
                            ),
                            true,
                        );
                        status_update = Some((
                            format!(
                                "TWAP slice {} status unknown; refreshing account data",
                                pending.index
                            ),
                            true,
                        ));
                        refresh_policy = TwapAccountRefresh::Immediate;
                    } else {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::NoFill;
                        }
                        twap.push_event(
                            TwapEventKind::Placed,
                            format!(
                                "Slice {} completed without fill: {summary_text}",
                                pending.index
                            ),
                            false,
                        );
                        status_update = Some((
                            format!("TWAP slice {} completed without fill", pending.index),
                            false,
                        ));
                    }
                }
                Err(error) => {
                    if let Some(child) = twap
                        .child_orders
                        .iter_mut()
                        .find(|child| child.index == pending.index)
                    {
                        child.status = TwapChildStatus::StatusUnknown;
                        child.exchange_summary = error.clone();
                    }
                    twap.status = TwapStatus::Error;
                    twap.push_event(
                        TwapEventKind::Error,
                        format!("Slice {} status unknown: {error}", pending.index),
                        true,
                    );
                    status_update = Some((
                        format!("TWAP slice {} status unknown: {error}", pending.index),
                        true,
                    ));
                }
            }
        }

        if let Some(status) = status_update {
            self.order_status = Some(status);
        }

        if let Some((key, asset, oid)) = cancel_unexpected {
            if key.is_empty() {
                if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
                    twap.status = TwapStatus::Error;
                    twap.push_event(
                        TwapEventKind::Error,
                        "Unexpected resting order could not be cancelled: original agent key unavailable"
                            .to_string(),
                        true,
                    );
                }
                self.order_status = Some((
                    "TWAP error: unexpected resting order could not be cancelled".into(),
                    true,
                ));
                return self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id);
            }
            let cancel_task = Task::perform(cancel_order(key.into(), asset, oid), move |result| {
                Message::TwapUnexpectedCancelResult {
                    twap_id,
                    oid,
                    result: Box::new(result),
                }
            });
            return if self.twap_refresh_policy_needs_refresh(refresh_policy, twap_id) {
                Task::batch([self.refresh_account_data(), cancel_task])
            } else {
                cancel_task
            };
        }

        self.finish_twap_attempt(twap_id, now);
        self.archive_twap_if_terminal(twap_id);
        self.refresh_after_twap_result(refresh_policy, twap_id)
    }

    pub(crate) fn handle_twap_unexpected_cancel_result(
        &mut self,
        twap_id: u64,
        oid: u64,
        result: Result<crate::signing::ExchangeResponse, String>,
    ) -> Task<Message> {
        let now = Instant::now();
        if let Some(twap) = self.twap_orders.get_mut(&twap_id)
            && matches!(
                twap.pending_op,
                Some(TwapPendingOp::CancelUnexpectedResting { oid: pending_oid })
                    if pending_oid == oid
            )
        {
            twap.pending_op = None;
            for child in &mut twap.child_orders {
                if child.oid == Some(oid) {
                    child.status = TwapChildStatus::UnexpectedRestingCancelled;
                    child.exchange_summary = match &result {
                        Ok(response) => response.summary(),
                        Err(error) => error.clone(),
                    };
                }
            }
            match result {
                Ok(response) if !response.is_error() => {
                    twap.push_event(
                        TwapEventKind::Stopped,
                        format!("Canceled unexpected resting child oid {oid}"),
                        false,
                    );
                }
                Ok(response) => {
                    twap.status = TwapStatus::Error;
                    twap.push_event(
                        TwapEventKind::Error,
                        format!(
                            "Failed to cancel unexpected resting child oid {oid}: {}",
                            response.summary()
                        ),
                        true,
                    );
                }
                Err(error) => {
                    twap.status = TwapStatus::Error;
                    twap.push_event(
                        TwapEventKind::Error,
                        format!("Cancel status unknown for unexpected child oid {oid}: {error}"),
                        true,
                    );
                }
            }
        }
        self.finish_twap_attempt(twap_id, now);
        self.archive_twap_if_terminal(twap_id);
        self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id)
    }

    pub(crate) fn reconcile_twap_fills_from_account(&mut self) {
        let Some(data) = self.account_data.as_ref() else {
            return;
        };
        let fills = data.fills.clone();
        let mut archive_ids = Vec::new();
        for twap in self.twap_orders.values_mut() {
            if self.connected_address.as_deref() != Some(twap.account_address.as_str()) {
                continue;
            }
            let before = twap.filled_size;
            let before_status = twap.status;
            twap.reconcile_fills(&fills);
            if twap.filled_size > before {
                twap.push_event(
                    TwapEventKind::Filled,
                    format!(
                        "Reconciled fills: {} / {} filled",
                        float_to_wire(twap.filled_size),
                        float_to_wire(twap.target_size)
                    ),
                    false,
                );
            }
            if twap.status.is_terminal()
                && (twap.filled_size > before || twap.status != before_status)
            {
                archive_ids.push(twap.id);
            }
        }
        for twap_id in archive_ids {
            self.archive_twap_if_terminal(twap_id);
        }
    }

    pub(crate) fn open_twap_details(&mut self, twap_id: u64) -> Task<Message> {
        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return Task::none();
        };
        if let Some(window_id) = twap.window_id {
            return window::gain_focus(window_id);
        }
        let settings = window::Settings {
            size: Size::new(760.0, 560.0),
            ..window::Settings::default()
        };
        let (window_id, task) = window::open(settings);
        twap.window_id = Some(window_id);
        task.map(Message::WindowOpened)
    }

    fn execute_due_twap_slice(&mut self, twap_id: u64, now: Instant) -> Task<Message> {
        if self.expire_twap_if_deadline_passed(twap_id, now) {
            return Task::none();
        }
        if let Some(twap) = self.twap_orders.get(&twap_id)
            && self.connected_address.as_deref() != Some(twap.account_address.as_str())
        {
            return self.stop_twap_with_reason(
                twap_id,
                "TWAP stopped: account changed before slice",
                true,
            );
        }

        let Some((book, book_updated_at, is_buy, min_price, max_price, sz_decimals, is_spot)) =
            self.twap_orders.get(&twap_id).and_then(|twap| {
                twap.latest_book.as_ref().map(|snapshot| {
                    (
                        snapshot.book.clone(),
                        snapshot.updated_at,
                        twap.is_buy,
                        twap.min_price,
                        twap.max_price,
                        twap.sz_decimals,
                        twap.is_spot,
                    )
                })
            })
        else {
            if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
                twap.status = TwapStatus::WaitingForMarket;
            }
            return Task::none();
        };

        if now.saturating_duration_since(book_updated_at) > TWAP_BOOK_STALE_AFTER {
            self.record_twap_skip(
                twap_id,
                now,
                TwapEventKind::SkippedStaleBook,
                "TWAP slice skipped: market data is stale".to_string(),
                true,
            );
            return Task::none();
        }

        if !self.can_send_advanced_exchange_request(now) {
            return Task::none();
        }

        let planned_size = {
            let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
                return Task::none();
            };
            let Some(raw_size) = twap.next_slice_size() else {
                self.finish_twap_attempt(twap_id, now);
                return Task::none();
            };
            let Some(size) =
                quantize_twap_slice_size(raw_size, twap.remaining_size, twap.sz_decimals)
            else {
                self.record_twap_skip(
                    twap_id,
                    now,
                    TwapEventKind::SkippedRange,
                    "TWAP slice skipped: rounded slice size is below the asset minimum precision"
                        .to_string(),
                    true,
                );
                return Task::none();
            };
            size
        };

        let Some(raw_limit_price) =
            twap_limit_price_for_slice(&book, is_buy, planned_size, min_price, max_price)
        else {
            self.record_twap_skip(
                twap_id,
                now,
                TwapEventKind::SkippedRange,
                format!(
                    "TWAP slice skipped: book cannot fill {} inside {}-{}",
                    float_to_wire(planned_size),
                    format_price(min_price),
                    format_price(max_price)
                ),
                false,
            );
            return Task::none();
        };
        let Some(limit_price) = twap_ioc_limit_price(
            raw_limit_price,
            is_buy,
            sz_decimals,
            is_spot,
            min_price,
            max_price,
        ) else {
            self.record_twap_skip(
                twap_id,
                now,
                TwapEventKind::SkippedRange,
                "TWAP slice skipped: rounded IOC price would no longer cross inside range"
                    .to_string(),
                false,
            );
            return Task::none();
        };
        if !twap_order_notional_meets_minimum(planned_size, limit_price) {
            self.record_twap_skip(
                twap_id,
                now,
                TwapEventKind::SkippedMinimum,
                format!(
                    "TWAP slice skipped: child notional ${:.2} is below Hyperliquid's ${MIN_EXCHANGE_ORDER_NOTIONAL_USD:.0} minimum",
                    planned_size * limit_price
                ),
                true,
            );
            return Task::none();
        }

        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return Task::none();
        };
        let key = twap.agent_key.trim().to_string();
        if key.is_empty() {
            return self.stop_twap_with_reason(
                twap_id,
                "TWAP stopped: original agent key is unavailable",
                true,
            );
        }

        let slice_index = twap.slices_attempted.saturating_add(1);
        twap.slices_attempted = slice_index;
        twap.slices_sent = twap.slices_sent.saturating_add(1);
        twap.pending_op = Some(TwapPendingOp::Place(TwapPendingSlice {
            index: slice_index,
            planned_size,
            limit_price,
        }));
        twap.status = TwapStatus::Running;
        twap.child_orders.push(TwapChildOrder {
            index: slice_index,
            requested_at: now,
            planned_size,
            limit_price,
            oid: None,
            status: TwapChildStatus::Pending,
            exchange_summary: "Placing".to_string(),
            filled_size: 0.0,
            avg_price: None,
            fee: 0.0,
        });
        twap.push_event(
            TwapEventKind::Placed,
            format!(
                "Slice {slice_index} placing {} @ {}",
                float_to_wire(planned_size),
                format_price(limit_price)
            ),
            false,
        );

        let asset = twap.asset;
        let reduce_only = twap.reduce_only;
        self.last_advanced_exchange_request_at = Some(now);

        Task::perform(
            place_order(
                key.into(),
                asset,
                is_buy,
                float_to_wire(limit_price),
                float_to_wire(planned_size),
                OrderKind::LimitIoc,
                reduce_only,
            ),
            move |result| Message::TwapSliceResult {
                twap_id,
                result: Box::new(result),
            },
        )
    }

    fn record_twap_skip(
        &mut self,
        twap_id: u64,
        now: Instant,
        kind: TwapEventKind,
        message: String,
        is_error: bool,
    ) {
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            twap.slices_attempted = twap.slices_attempted.saturating_add(1);
            twap.push_event(kind, message.clone(), is_error);
            self.order_status = Some((message, is_error));
            twap.schedule_after_attempt(now);
        }
    }

    fn finish_twap_attempt(&mut self, twap_id: u64, now: Instant) {
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            if twap.stop_requested {
                let (message, is_error) = twap
                    .stop_reason
                    .clone()
                    .unwrap_or_else(|| ("TWAP stopped".to_string(), false));
                twap.status = TwapStatus::Stopped;
                twap.push_event(TwapEventKind::Stopped, message.clone(), is_error);
                self.order_status = Some((message, is_error));
            } else if twap.pending_op.is_none() && !twap.status.is_terminal() {
                twap.schedule_after_attempt(now);
            }
        }
        self.archive_twap_if_terminal(twap_id);
    }

    fn expire_twap_if_deadline_passed(&mut self, twap_id: u64, now: Instant) -> bool {
        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return true;
        };
        if now < twap.ends_at || twap.pending_op.is_some() || twap.status.is_terminal() {
            return false;
        }
        twap.slices_attempted = twap.slice_count;
        twap.status = if twap.filled_size > 0.0 {
            TwapStatus::CompletedPartial
        } else {
            TwapStatus::Stopped
        };
        let message = if twap.filled_size > 0.0 {
            "TWAP ended at deadline with unfilled remainder".to_string()
        } else {
            "TWAP ended at deadline without fills".to_string()
        };
        twap.push_event(TwapEventKind::Completed, message.clone(), false);
        self.order_status = Some((message, false));
        self.archive_twap_if_terminal(twap_id);
        true
    }

    fn refresh_after_twap_result(
        &mut self,
        policy: TwapAccountRefresh,
        twap_id: u64,
    ) -> Task<Message> {
        match policy {
            TwapAccountRefresh::Immediate => {
                let Some(addr) = self.connected_address.clone() else {
                    return Task::none();
                };
                self.force_refresh_account_data_for_reconciliation(addr)
            }
            _ if self.twap_refresh_policy_needs_refresh(policy, twap_id) => {
                self.refresh_account_data()
            }
            _ => Task::none(),
        }
    }

    fn twap_refresh_policy_needs_refresh(&self, policy: TwapAccountRefresh, twap_id: u64) -> bool {
        let twap_is_terminal = self
            .twap_orders
            .get(&twap_id)
            .is_some_and(|twap| twap.status.is_terminal());
        policy.should_refresh(twap_is_terminal)
    }

    fn can_send_advanced_exchange_request(&self, now: Instant) -> bool {
        self.last_advanced_exchange_request_at.is_none_or(|last| {
            now.saturating_duration_since(last) >= ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL
        })
    }

    fn effective_twap_price_range(
        &mut self,
        sz_decimals: u32,
        is_spot: bool,
    ) -> Option<(f64, f64)> {
        let parsed_min = parse_positive_price(&self.twap_form.min_price);
        let parsed_max = parse_positive_price(&self.twap_form.max_price);
        let mut min_price = parsed_min;
        let mut max_price = parsed_max;

        if min_price.is_none() || max_price.is_none() {
            let mid = self.resolve_mid_for_symbol(&self.active_symbol)?;
            if !mid.is_finite() || mid <= 0.0 {
                return None;
            }
            let width = self.market_slippage_fraction().max(0.001);
            if min_price.is_none() {
                min_price = Some(round_price(mid * (1.0 - width), sz_decimals, is_spot));
            }
            if max_price.is_none() {
                max_price = Some(round_price(mid * (1.0 + width), sz_decimals, is_spot));
            }
        }

        let min_price = min_price?;
        let max_price = max_price?;
        if !min_price.is_finite()
            || !max_price.is_finite()
            || min_price <= 0.0
            || max_price <= min_price
        {
            return None;
        }
        self.twap_form.min_price = format_price(min_price);
        self.twap_form.max_price = format_price(max_price);
        Some((min_price, max_price))
    }
}

fn parse_positive_price(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn twap_place_result_refresh_policy(
    result: &Result<crate::signing::ExchangeResponse, String>,
) -> TwapAccountRefresh {
    match result {
        Err(_) => TwapAccountRefresh::Immediate,
        Ok(response) if response.is_ambiguous_order_result() => TwapAccountRefresh::Immediate,
        Ok(response) if response.is_error() => TwapAccountRefresh::None,
        Ok(_) => TwapAccountRefresh::OnTerminal,
    }
}

#[cfg(test)]
mod tests {
    use super::{TwapAccountRefresh, twap_ioc_limit_price, twap_place_result_refresh_policy};
    use crate::signing::ExchangeResponse;

    fn exchange_response(status: serde_json::Value) -> ExchangeResponse {
        serde_json::from_value(serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [status]
                }
            }
        }))
        .expect("test exchange response should deserialize")
    }

    #[test]
    fn twap_place_refresh_policy_reconciles_only_unknown_or_terminal_results() {
        let unknown: Result<ExchangeResponse, String> =
            Err("Exchange request failed after submit".to_string());
        assert_eq!(
            twap_place_result_refresh_policy(&unknown),
            TwapAccountRefresh::Immediate
        );

        let rejected = Ok(exchange_response(serde_json::json!({
            "error": "Order must have minimum value of $10"
        })));
        assert_eq!(
            twap_place_result_refresh_policy(&rejected),
            TwapAccountRefresh::None
        );

        let filled = Ok(exchange_response(serde_json::json!({
            "filled": {
                "totalSz": "1.25",
                "avgPx": "100",
                "oid": 77_u64
            }
        })));
        assert_eq!(
            twap_place_result_refresh_policy(&filled),
            TwapAccountRefresh::OnTerminal
        );

        let ambiguous: Result<ExchangeResponse, String> =
            Ok(serde_json::from_value(serde_json::json!({
                "status": "ok",
                "response": {
                    "type": "order",
                    "data": {
                        "statuses": "schema-shifted"
                    }
                }
            }))
            .expect("ambiguous exchange response should deserialize"));
        assert_eq!(
            twap_place_result_refresh_policy(&ambiguous),
            TwapAccountRefresh::Immediate
        );

        assert!(!TwapAccountRefresh::OnTerminal.should_refresh(false));
        assert!(TwapAccountRefresh::OnTerminal.should_refresh(true));
        assert!(TwapAccountRefresh::Immediate.should_refresh(false));
    }

    #[test]
    fn twap_ioc_limit_price_preserves_marketability_after_rounding() {
        assert_eq!(
            twap_ioc_limit_price(1.2344, true, 3, false, 1.0, 2.0),
            Some(1.2344)
        );
        assert_eq!(
            twap_ioc_limit_price(1.2346, false, 3, false, 1.0, 2.0),
            Some(1.2346)
        );
        assert_eq!(
            twap_ioc_limit_price(100.0, true, 3, false, 99.0, 100.0),
            Some(100.0)
        );
        assert_eq!(
            twap_ioc_limit_price(100.1, true, 3, false, 99.0, 100.0),
            None
        );
    }
}
