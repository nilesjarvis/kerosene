use crate::api::MarketType;
use crate::api::fetch_order_status_by_cloid;
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use crate::signing::{
    OrderKind, PlaceOrderRequest, cancel_order, cancel_order_by_cloid, float_to_wire,
    place_order_with_cloid, round_price,
};
use crate::twap_state::{
    ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL, MAX_ACTIVE_ADVANCED_ORDERS,
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_BOOK_STALE_AFTER, TWAP_MAX_RETRY_ATTEMPTS,
    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES, TWAP_RECONCILIATION_TIMEOUT, TwapBookSnapshot,
    TwapChildOrder, TwapChildStatus, TwapEventKind, TwapOrder, TwapPauseReason, TwapPendingOp,
    TwapPendingSlice, TwapStatus, parse_twap_duration_minutes, parse_twap_slice_count,
    quantize_twap_slice_size, twap_aggregate_schedule_has_capacity, twap_aggregate_slice_rate,
    twap_child_cloid, twap_limit_price_for_slice, twap_min_quantized_child_notional,
    twap_order_notional_meets_minimum, twap_required_slice_rate, twap_response_fill_summary,
    twap_target_size_from_quantity, validate_twap_interval,
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
        if twap.status == TwapStatus::Paused
            && twap.pause_reason == Some(TwapPauseReason::StaleMarketData)
        {
            twap.clear_pause();
            twap.push_event(
                TwapEventKind::Reconciled,
                "TWAP resumed: market data is fresh".to_string(),
                false,
            );
        } else if twap.status == TwapStatus::WaitingForMarket {
            twap.status = TwapStatus::Running;
        }
        Task::none()
    }

    pub(crate) fn handle_twap_tick(&mut self) -> Task<Message> {
        let now = Instant::now();
        if let Some(twap_id) = self
            .twap_orders
            .iter()
            .find(|(_, twap)| {
                !twap.status.is_terminal() && twap.pending_op.is_none() && now >= twap.ends_at
            })
            .map(|(id, _)| *id)
        {
            self.expire_twap_if_deadline_passed(twap_id, now);
            return Task::none();
        }
        let Some(twap_id) = self
            .twap_orders
            .iter()
            .filter(|(_, twap)| twap.can_schedule_at(now))
            .min_by_key(|(id, twap)| (twap.next_slice_due, *id))
            .map(|(id, _)| *id)
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
            .and_then(|twap| match &twap.pending_op {
                Some(TwapPendingOp::Place(slice)) => Some(slice.clone()),
                _ => None,
            });
        let Some(pending) = pending else {
            return self.refresh_after_twap_result(refresh_policy, twap_id);
        };

        let mut status_update = None;
        let mut cancel_unexpected = None;
        let mut status_check_cloid = None;
        let mut finish_attempt = true;
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
                        child.cloid = Some(pending.cloid.clone());
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
                        twap.retry_slice = None;
                    } else if response.is_error() {
                        match classify_twap_exchange_error(&summary_text) {
                            TwapExchangeErrorAction::Retry(reason) => {
                                finish_attempt = false;
                                refresh_policy = TwapAccountRefresh::None;
                                let retry_count = pending.retry_count.saturating_add(1);
                                if retry_count > TWAP_MAX_RETRY_ATTEMPTS {
                                    if let Some(child) = twap
                                        .child_orders
                                        .iter_mut()
                                        .find(|child| child.index == pending.index)
                                    {
                                        child.status = TwapChildStatus::Rejected;
                                        child.retry_count = retry_count;
                                    }
                                    twap.status = TwapStatus::Error;
                                    twap.push_event(
                                        TwapEventKind::Error,
                                        format!(
                                            "Slice {} stopped after {} retry attempts: {summary_text}",
                                            pending.index, TWAP_MAX_RETRY_ATTEMPTS
                                        ),
                                        true,
                                    );
                                    status_update = Some((
                                        format!(
                                            "TWAP slice {} stopped after retry budget: {summary_text}",
                                            pending.index
                                        ),
                                        true,
                                    ));
                                } else {
                                    let mut retry_slice = pending.clone();
                                    retry_slice.retry_count = retry_count;
                                    twap.retry_slice = Some(retry_slice);
                                    if let Some(child) = twap
                                        .child_orders
                                        .iter_mut()
                                        .find(|child| child.index == pending.index)
                                    {
                                        child.status = TwapChildStatus::Retrying;
                                        child.retry_count = retry_count;
                                        child.exchange_summary = summary_text.clone();
                                    }
                                    let delay = TwapOrder::retry_delay(retry_count);
                                    twap.pause(
                                        reason,
                                        Some(now + delay),
                                        format!(
                                            "Slice {} paused: {}; retry {}/{} in about {}s",
                                            pending.index,
                                            reason.label(),
                                            retry_count,
                                            TWAP_MAX_RETRY_ATTEMPTS,
                                            delay.as_secs()
                                        ),
                                        true,
                                    );
                                    status_update = Some((
                                        format!(
                                            "TWAP paused: {}; retry {}/{} in about {}s",
                                            reason.label(),
                                            retry_count,
                                            TWAP_MAX_RETRY_ATTEMPTS,
                                            delay.as_secs()
                                        ),
                                        true,
                                    ));
                                }
                            }
                            TwapExchangeErrorAction::Terminal => {
                                finish_attempt = false;
                                if let Some(child) = twap
                                    .child_orders
                                    .iter_mut()
                                    .find(|child| child.index == pending.index)
                                {
                                    child.status = TwapChildStatus::Rejected;
                                }
                                twap.status = TwapStatus::Error;
                                twap.push_event(
                                    TwapEventKind::Rejected,
                                    format!("Slice {} rejected: {summary_text}", pending.index),
                                    true,
                                );
                                status_update = Some((
                                    format!(
                                        "TWAP stopped: slice {} rejected: {summary_text}",
                                        pending.index
                                    ),
                                    true,
                                ));
                            }
                            TwapExchangeErrorAction::ConsumeSlice => {
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
                                    format!(
                                        "TWAP slice {} rejected: {summary_text}",
                                        pending.index
                                    ),
                                    true,
                                ));
                                twap.retry_slice = None;
                            }
                        }
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
                        twap.retry_slice = None;
                    } else if response.is_fully_filled() {
                        finish_attempt = false;
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::AwaitingReconciliation;
                        }
                        twap.status_check_cloid = Some(pending.cloid.clone());
                        twap.pause(
                            TwapPauseReason::StatusUnknown,
                            None,
                            format!(
                                "Slice {} reported filled but fill size was unavailable; checking status",
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
                        status_check_cloid = Some(pending.cloid.clone());
                    } else if let Some(oid) = oid {
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::UnexpectedResting;
                        }
                        twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
                            oid: Some(oid),
                            cloid: Some(pending.cloid.clone()),
                        });
                        twap.pause(
                            TwapPauseReason::UnexpectedResting,
                            None,
                            format!(
                                "Slice {} unexpectedly rested as oid {oid}; cancelling",
                                pending.index
                            ),
                            true,
                        );
                        cancel_unexpected = Some((
                            twap.agent_key.trim().to_string(),
                            twap.asset,
                            Some(oid),
                            Some(pending.cloid.clone()),
                        ));
                        status_update = Some((
                            format!(
                                "TWAP slice {} unexpectedly rested; cancelling",
                                pending.index
                            ),
                            true,
                        ));
                        finish_attempt = false;
                    } else if response.is_ambiguous_order_result() {
                        finish_attempt = false;
                        if let Some(child) = twap
                            .child_orders
                            .iter_mut()
                            .find(|child| child.index == pending.index)
                        {
                            child.status = TwapChildStatus::StatusUnknown;
                        }
                        twap.status_check_cloid = Some(pending.cloid.clone());
                        twap.pause(
                            TwapPauseReason::StatusUnknown,
                            None,
                            format!(
                                "Slice {} returned ambiguous order status: {summary_text}; checking status",
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
                        status_check_cloid = Some(pending.cloid.clone());
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
                        twap.retry_slice = None;
                    }
                }
                Err(error) => {
                    finish_attempt = false;
                    if let Some(child) = twap
                        .child_orders
                        .iter_mut()
                        .find(|child| child.index == pending.index)
                    {
                        child.status = TwapChildStatus::StatusUnknown;
                        child.exchange_summary = error.clone();
                        child.cloid = Some(pending.cloid.clone());
                    }
                    twap.status_check_cloid = Some(pending.cloid.clone());
                    twap.pause(
                        TwapPauseReason::StatusUnknown,
                        None,
                        format!(
                            "Slice {} status unknown after transport error: {error}; checking status",
                            pending.index
                        ),
                        true,
                    );
                    status_update = Some((
                        format!("TWAP slice {} status unknown: {error}", pending.index),
                        true,
                    ));
                    refresh_policy = TwapAccountRefresh::Immediate;
                    status_check_cloid = Some(pending.cloid.clone());
                }
            }
        }

        if let Some(status) = status_update {
            self.order_status = Some(status);
        }

        if let Some((key, asset, oid, cloid)) = cancel_unexpected {
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
            let cancel_task = twap_cancel_child_task(twap_id, key, asset, oid, cloid);
            return if self.twap_refresh_policy_needs_refresh(refresh_policy, twap_id) {
                Task::batch([self.refresh_account_data(), cancel_task])
            } else {
                cancel_task
            };
        }

        if let Some(cloid) = status_check_cloid {
            let status_task = self.check_twap_child_status(twap_id, cloid);
            return if self.twap_refresh_policy_needs_refresh(refresh_policy, twap_id) {
                Task::batch([self.refresh_account_data(), status_task])
            } else {
                status_task
            };
        }

        if finish_attempt {
            self.finish_twap_attempt(twap_id, now);
        }
        self.archive_twap_if_terminal(twap_id);
        self.refresh_after_twap_result(refresh_policy, twap_id)
    }

    pub(crate) fn handle_twap_unexpected_cancel_result(
        &mut self,
        twap_id: u64,
        oid: Option<u64>,
        cloid: Option<String>,
        result: Result<crate::signing::ExchangeResponse, String>,
    ) -> Task<Message> {
        let now = Instant::now();
        let mut retry_cancel = None;
        let mut finish_attempt = true;
        if let Some(twap) = self.twap_orders.get_mut(&twap_id)
            && matches!(
                &twap.pending_op,
                Some(TwapPendingOp::CancelUnexpectedResting {
                    oid: pending_oid,
                    cloid: pending_cloid,
                }) if twap_cancel_target_matches(
                    *pending_oid,
                    pending_cloid.as_deref(),
                    oid,
                    cloid.as_deref(),
                )
            )
        {
            let exchange_summary = match &result {
                Ok(response) => response.summary(),
                Err(error) => error.clone(),
            };
            for child in &mut twap.child_orders {
                if twap_child_matches_cancel_target(child, oid, cloid.as_deref()) {
                    child.exchange_summary = exchange_summary.clone();
                }
            }
            match result {
                Ok(response) if !response.is_error() => {
                    twap.pending_op = None;
                    twap.cancel_retries = 0;
                    for child in &mut twap.child_orders {
                        if twap_child_matches_cancel_target(child, oid, cloid.as_deref()) {
                            child.status = TwapChildStatus::UnexpectedRestingCancelled;
                        }
                    }
                    twap.clear_pause();
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!(
                            "Canceled unexpected resting child {}",
                            twap_cancel_label(oid, cloid.as_deref())
                        ),
                        false,
                    );
                }
                Ok(response) => {
                    let summary = response.summary();
                    if twap_terminal_cancel_error(&summary) {
                        twap.pending_op = None;
                        twap.cancel_retries = 0;
                        for child in &mut twap.child_orders {
                            if twap_child_matches_cancel_target(child, oid, cloid.as_deref()) {
                                child.status = TwapChildStatus::UnexpectedRestingCancelled;
                            }
                        }
                        twap.clear_pause();
                        twap.push_event(
                            TwapEventKind::Reconciled,
                            format!(
                                "Unexpected resting child {} is no longer open: {summary}",
                                twap_cancel_label(oid, cloid.as_deref())
                            ),
                            true,
                        );
                    } else {
                        finish_attempt = false;
                        twap.cancel_retries = twap.cancel_retries.saturating_add(1);
                        if twap.cancel_retries >= TWAP_MAX_UNEXPECTED_CANCEL_RETRIES {
                            twap.pending_op = None;
                            twap.status = TwapStatus::Error;
                            twap.push_event(
                                TwapEventKind::Error,
                                format!(
                                    "Failed to cancel unexpected resting child {} after {} attempts: {summary}",
                                    twap_cancel_label(oid, cloid.as_deref()),
                                    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES
                                ),
                                true,
                            );
                        } else {
                            let delay = TwapOrder::retry_delay(twap.cancel_retries);
                            twap.pause(
                                TwapPauseReason::UnexpectedResting,
                                Some(now + delay),
                                format!(
                                    "Cancel retry {}/{} for unexpected resting child {} in about {}s",
                                    twap.cancel_retries,
                                    TWAP_MAX_UNEXPECTED_CANCEL_RETRIES,
                                    twap_cancel_label(oid, cloid.as_deref()),
                                    delay.as_secs()
                                ),
                                true,
                            );
                            retry_cancel = Some((
                                twap.agent_key.trim().to_string(),
                                twap.asset,
                                oid,
                                cloid.clone(),
                            ));
                        }
                    }
                }
                Err(error) => {
                    finish_attempt = false;
                    twap.cancel_retries = twap.cancel_retries.saturating_add(1);
                    if twap.cancel_retries >= TWAP_MAX_UNEXPECTED_CANCEL_RETRIES {
                        twap.pending_op = None;
                        twap.status = TwapStatus::Error;
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                "Cancel status unknown for unexpected child {} after {} attempts: {error}",
                                twap_cancel_label(oid, cloid.as_deref()),
                                TWAP_MAX_UNEXPECTED_CANCEL_RETRIES
                            ),
                            true,
                        );
                    } else {
                        let delay = TwapOrder::retry_delay(twap.cancel_retries);
                        twap.pause(
                            TwapPauseReason::UnexpectedResting,
                            Some(now + delay),
                            format!(
                                "Cancel status unknown for unexpected child {}; retry {}/{} in about {}s",
                                twap_cancel_label(oid, cloid.as_deref()),
                                twap.cancel_retries,
                                TWAP_MAX_UNEXPECTED_CANCEL_RETRIES,
                                delay.as_secs()
                            ),
                            true,
                        );
                        retry_cancel = Some((
                            twap.agent_key.trim().to_string(),
                            twap.asset,
                            oid,
                            cloid.clone(),
                        ));
                    }
                }
            }
        }

        if let Some((key, asset, oid, cloid)) = retry_cancel {
            return twap_cancel_child_task(twap_id, key, asset, oid, cloid);
        }
        if finish_attempt {
            self.finish_twap_attempt(twap_id, now);
        }
        self.archive_twap_if_terminal(twap_id);
        self.refresh_after_twap_result(TwapAccountRefresh::Immediate, twap_id)
    }

    pub(crate) fn handle_twap_order_status_result(
        &mut self,
        twap_id: u64,
        cloid: String,
        result: Result<crate::api::OrderStatusResult, String>,
    ) -> Task<Message> {
        let now = Instant::now();
        let mut cancel_unexpected = None;
        let mut refresh = false;
        let mut retry_status_check = None;
        let mut finish_attempt = false;

        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            if twap.status_check_cloid.as_deref() != Some(cloid.as_str()) {
                return Task::none();
            }

            match result {
                Ok(status) if status.is_missing() || status.is_no_fill_terminal() => {
                    twap.status_check_cloid = None;
                    twap.status_check_retries = 0;
                    twap.retry_slice = None;
                    for child in &mut twap.child_orders {
                        if child.cloid.as_deref() == Some(cloid.as_str()) {
                            child.oid = status.oid.or(child.oid);
                            child.status = if status.is_missing() {
                                TwapChildStatus::NoFill
                            } else {
                                TwapChildStatus::Rejected
                            };
                            child.exchange_summary = status.raw_summary.clone();
                        }
                    }
                    twap.clear_pause();
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!("Slice status reconciled: {}", status.raw_summary),
                        status.is_no_fill_terminal(),
                    );
                    self.order_status = Some((
                        format!("TWAP status reconciled: {}", status.raw_summary),
                        false,
                    ));
                    finish_attempt = true;
                }
                Ok(status) if status.is_open() => {
                    twap.status_check_cloid = None;
                    twap.status_check_retries = 0;
                    for child in &mut twap.child_orders {
                        if child.cloid.as_deref() == Some(cloid.as_str()) {
                            child.oid = status.oid.or(child.oid);
                            child.status = TwapChildStatus::UnexpectedResting;
                            child.exchange_summary = status.raw_summary.clone();
                        }
                    }
                    twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
                        oid: status.oid,
                        cloid: Some(cloid.clone()),
                    });
                    twap.pause(
                        TwapPauseReason::UnexpectedResting,
                        None,
                        format!("Slice unexpectedly open after status check; cancelling {cloid}"),
                        true,
                    );
                    cancel_unexpected = Some((
                        twap.agent_key.trim().to_string(),
                        twap.asset,
                        status.oid,
                        Some(cloid.clone()),
                    ));
                }
                Ok(status) if status.is_filled() => {
                    for child in &mut twap.child_orders {
                        if child.cloid.as_deref() == Some(cloid.as_str()) {
                            child.oid = status.oid.or(child.oid);
                            child.status = TwapChildStatus::AwaitingReconciliation;
                            child.exchange_summary = status.raw_summary.clone();
                        }
                    }
                    twap.reconciliation_deadline = Some(now + TWAP_RECONCILIATION_TIMEOUT);
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        format!(
                            "Slice {} is filled on exchange; refreshing account fills",
                            cloid
                        ),
                        false,
                    );
                    self.order_status = Some((
                        "TWAP child filled on exchange; refreshing account fills".to_string(),
                        false,
                    ));
                    refresh = true;
                }
                Ok(status) => {
                    twap.status_check_retries = twap.status_check_retries.saturating_add(1);
                    if twap.status_check_retries >= TWAP_MAX_RETRY_ATTEMPTS {
                        twap.status_check_cloid = None;
                        twap.status = TwapStatus::Error;
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                "Could not reconcile slice {cloid} after status '{}'",
                                status.status
                            ),
                            true,
                        );
                    } else {
                        let delay = TwapOrder::retry_delay(twap.status_check_retries);
                        twap.pause(
                            TwapPauseReason::StatusUnknown,
                            Some(now + delay),
                            format!(
                                "Slice status still unclear ({}); retry {}/{} in about {}s",
                                status.status,
                                twap.status_check_retries,
                                TWAP_MAX_RETRY_ATTEMPTS,
                                delay.as_secs()
                            ),
                            true,
                        );
                        retry_status_check = Some((cloid.clone(), delay));
                    }
                }
                Err(error) => {
                    twap.status_check_retries = twap.status_check_retries.saturating_add(1);
                    if twap.status_check_retries >= TWAP_MAX_RETRY_ATTEMPTS {
                        twap.status_check_cloid = None;
                        twap.status = TwapStatus::Error;
                        twap.push_event(
                            TwapEventKind::Error,
                            format!(
                                "Could not check slice status after {} attempts: {error}",
                                TWAP_MAX_RETRY_ATTEMPTS
                            ),
                            true,
                        );
                    } else {
                        let delay = TwapOrder::retry_delay(twap.status_check_retries);
                        twap.pause(
                            TwapPauseReason::NetworkError,
                            Some(now + delay),
                            format!(
                                "Slice status check failed; retry {}/{} in about {}s: {error}",
                                twap.status_check_retries,
                                TWAP_MAX_RETRY_ATTEMPTS,
                                delay.as_secs()
                            ),
                            true,
                        );
                        retry_status_check = Some((cloid.clone(), delay));
                    }
                }
            }
        }

        if let Some((key, asset, oid, cloid)) = cancel_unexpected {
            return twap_cancel_child_task(twap_id, key, asset, oid, cloid);
        }
        if let Some((cloid, delay)) = retry_status_check {
            return self.check_twap_child_status_after(twap_id, cloid, delay);
        }
        if finish_attempt {
            self.finish_twap_attempt(twap_id, now);
        }
        self.archive_twap_if_terminal(twap_id);
        if refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }

    pub(crate) fn reconcile_twap_fills_from_account(&mut self) {
        let Some(data) = self.account_data.as_ref() else {
            return;
        };
        let now = Instant::now();
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
                if before_status == TwapStatus::Paused && !twap.has_status_unknown_child() {
                    twap.status_check_cloid = None;
                    twap.reconciliation_deadline = None;
                    twap.push_event(
                        TwapEventKind::Reconciled,
                        "TWAP resumed after account fill reconciliation".to_string(),
                        false,
                    );
                }
                twap.push_event(
                    TwapEventKind::Filled,
                    format!(
                        "Reconciled fills: {} / {} filled",
                        float_to_wire(twap.filled_size),
                        float_to_wire(twap.target_size)
                    ),
                    false,
                );
            } else if TwapOrder::reconciliation_timed_out(twap.reconciliation_deadline, now)
                && twap.has_status_unknown_child()
            {
                // The exchange reported a child as filled, but `account.fills`
                // never echoed it within TWAP_RECONCILIATION_TIMEOUT. Tear
                // the TWAP down with a clear, actionable error rather than
                // leave it paused indefinitely with `status_check_cloid` set
                // (which would block every future slice via `can_schedule_at`).
                let pending_cloid = twap.status_check_cloid.clone().unwrap_or_default();
                twap.status_check_cloid = None;
                twap.reconciliation_deadline = None;
                twap.status = TwapStatus::Error;
                twap.push_event(
                    TwapEventKind::Error,
                    format!(
                        "Could not reconcile slice {pending_cloid} via account fills within {}s; \
                         exchange reported fill but account fills did not catch up. Check the \
                         exchange before manually resuming.",
                        TWAP_RECONCILIATION_TIMEOUT.as_secs()
                    ),
                    true,
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
        // Defense in depth against the mute risk control. The mute handler
        // already stops matching TWAPs (see
        // `update_muted_ticker_preferences`), but this catches:
        //   (a) a mute that was applied between the schedule tick and this
        //       execute call;
        //   (b) any future code path that adds a TWAP without going through
        //       the mute eviction pass.
        // Without it, a freshly-muted symbol could fire one more slice off
        // its cached `latest_book` before the stop landed.
        if let Some(twap) = self.twap_orders.get(&twap_id)
            && self.is_ticker_muted(&twap.coin)
        {
            return self.stop_twap_with_reason(twap_id, "TWAP stopped: ticker was muted", false);
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
            if let Some(twap) = self.twap_orders.get_mut(&twap_id)
                && twap.status != TwapStatus::Paused
            {
                twap.status = TwapStatus::WaitingForMarket;
            }
            return Task::none();
        };

        if now.saturating_duration_since(book_updated_at) > TWAP_BOOK_STALE_AFTER {
            if let Some(twap) = self.twap_orders.get_mut(&twap_id)
                && twap.pause_reason != Some(TwapPauseReason::StaleMarketData)
            {
                twap.pause(
                    TwapPauseReason::StaleMarketData,
                    None,
                    "TWAP paused: market data is stale".to_string(),
                    true,
                );
                self.order_status = Some(("TWAP paused: market data is stale".to_string(), true));
            }
            return Task::none();
        }

        if !self.can_send_advanced_exchange_request(now) {
            return Task::none();
        }

        let retry_slice = self
            .twap_orders
            .get(&twap_id)
            .and_then(|twap| twap.retry_slice.clone());
        let planned_size = if let Some(slice) = &retry_slice {
            slice.planned_size
        } else {
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
            let message = format!(
                "TWAP slice skipped: book cannot fill {} inside {}-{}",
                float_to_wire(planned_size),
                format_price(min_price),
                format_price(max_price)
            );
            if let Some(slice) = &retry_slice {
                self.record_twap_retry_skip(
                    twap_id,
                    now,
                    slice.index,
                    TwapEventKind::SkippedRange,
                    message,
                    false,
                );
            } else {
                self.record_twap_skip(twap_id, now, TwapEventKind::SkippedRange, message, false);
            }
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
            let message =
                "TWAP slice skipped: rounded IOC price would no longer cross inside range"
                    .to_string();
            if let Some(slice) = &retry_slice {
                self.record_twap_retry_skip(
                    twap_id,
                    now,
                    slice.index,
                    TwapEventKind::SkippedRange,
                    message,
                    false,
                );
            } else {
                self.record_twap_skip(twap_id, now, TwapEventKind::SkippedRange, message, false);
            }
            return Task::none();
        };
        if !twap_order_notional_meets_minimum(planned_size, limit_price) {
            let message = format!(
                "TWAP slice skipped: child notional ${:.2} is below Hyperliquid's ${MIN_EXCHANGE_ORDER_NOTIONAL_USD:.0} minimum",
                planned_size * limit_price
            );
            if let Some(slice) = &retry_slice {
                self.record_twap_retry_skip(
                    twap_id,
                    now,
                    slice.index,
                    TwapEventKind::SkippedMinimum,
                    message,
                    true,
                );
            } else {
                self.record_twap_skip(twap_id, now, TwapEventKind::SkippedMinimum, message, true);
            }
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

        let pending_slice = if let Some(mut slice) = retry_slice {
            slice.limit_price = limit_price;
            twap.retry_slice = None;
            twap.pending_op = Some(TwapPendingOp::Place(slice.clone()));
            twap.status = TwapStatus::Running;
            twap.pause_reason = None;
            twap.paused_until = None;
            if let Some(child) = twap
                .child_orders
                .iter_mut()
                .find(|child| child.index == slice.index)
            {
                child.status = TwapChildStatus::Pending;
                child.limit_price = limit_price;
                child.retry_count = slice.retry_count;
                child.exchange_summary = format!("Retry {}", slice.retry_count);
            }
            twap.push_event(
                TwapEventKind::Retrying,
                format!(
                    "Slice {} retry {} placing {} @ {}",
                    slice.index,
                    slice.retry_count,
                    float_to_wire(planned_size),
                    format_price(limit_price)
                ),
                false,
            );
            slice
        } else {
            let slice_index = twap.slices_attempted.saturating_add(1);
            let cloid = twap_child_cloid(
                &twap.account_address,
                twap.id,
                twap.started_at_ms,
                slice_index,
            );
            twap.slices_attempted = slice_index;
            twap.slices_sent = twap.slices_sent.saturating_add(1);
            let slice = TwapPendingSlice {
                index: slice_index,
                planned_size,
                limit_price,
                cloid: cloid.clone(),
                retry_count: 0,
            };
            twap.pending_op = Some(TwapPendingOp::Place(slice.clone()));
            twap.status = TwapStatus::Running;
            twap.child_orders.push(TwapChildOrder {
                index: slice_index,
                requested_at: now,
                planned_size,
                limit_price,
                oid: None,
                cloid: Some(cloid),
                status: TwapChildStatus::Pending,
                exchange_summary: "Placing".to_string(),
                filled_size: 0.0,
                avg_price: None,
                fee: 0.0,
                retry_count: 0,
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
            slice
        };

        let asset = twap.asset;
        let reduce_only = twap.reduce_only;
        self.last_advanced_exchange_request_at = Some(now);

        Task::perform(
            place_order_with_cloid(
                key.into(),
                PlaceOrderRequest {
                    asset,
                    is_buy,
                    price: float_to_wire(limit_price),
                    size: float_to_wire(planned_size),
                    order_kind: OrderKind::LimitIoc,
                    reduce_only,
                    cloid: Some(pending_slice.cloid),
                },
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

    fn record_twap_retry_skip(
        &mut self,
        twap_id: u64,
        now: Instant,
        slice_index: u32,
        kind: TwapEventKind,
        message: String,
        is_error: bool,
    ) {
        if let Some(twap) = self.twap_orders.get_mut(&twap_id) {
            twap.retry_slice = None;
            if let Some(child) = twap
                .child_orders
                .iter_mut()
                .find(|child| child.index == slice_index)
            {
                child.status = TwapChildStatus::NoFill;
                child.exchange_summary = message.clone();
            }
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

    fn check_twap_child_status(&mut self, twap_id: u64, cloid: String) -> Task<Message> {
        let Some(address) = self.connected_address.clone() else {
            return Task::none();
        };
        let request_cloid = cloid.clone();
        Task::perform(
            fetch_order_status_by_cloid(address, request_cloid),
            move |result| Message::TwapOrderStatusLoaded {
                twap_id,
                cloid: cloid.clone(),
                result: Box::new(result),
            },
        )
    }

    fn check_twap_child_status_after(
        &mut self,
        twap_id: u64,
        cloid: String,
        delay: std::time::Duration,
    ) -> Task<Message> {
        let Some(address) = self.connected_address.clone() else {
            return Task::none();
        };
        let request_cloid = cloid.clone();
        Task::perform(
            async move {
                tokio::time::sleep(delay).await;
                fetch_order_status_by_cloid(address, request_cloid).await
            },
            move |result| Message::TwapOrderStatusLoaded {
                twap_id,
                cloid: cloid.clone(),
                result: Box::new(result),
            },
        )
    }

    fn twap_refresh_policy_needs_refresh(&self, policy: TwapAccountRefresh, twap_id: u64) -> bool {
        let twap_is_terminal = self
            .twap_orders
            .get(&twap_id)
            .is_some_and(|twap| twap.status.is_terminal());
        policy.should_refresh(twap_is_terminal)
    }

    fn can_send_advanced_exchange_request(&self, now: Instant) -> bool {
        !self.account_loading
            && !self.account_reconciliation_required
            && self.last_advanced_exchange_request_at.is_none_or(|last| {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TwapExchangeErrorAction {
    Retry(TwapPauseReason),
    Terminal,
    ConsumeSlice,
}

fn classify_twap_exchange_error(summary: &str) -> TwapExchangeErrorAction {
    let summary = summary.to_ascii_lowercase();
    if summary.contains("rate limit")
        || summary.contains("ratelimit")
        || summary.contains("too many requests")
        || summary.contains("429")
        || summary.contains("temporarily")
        || summary.contains("unavailable")
        || summary.contains("overloaded")
        || summary.contains("try again")
    {
        return TwapExchangeErrorAction::Retry(TwapPauseReason::RateLimited);
    }

    if summary.contains("signature")
        || summary.contains("agent")
        || summary.contains("unauthorized")
        || summary.contains("not approved")
        || summary.contains("minimum")
        || summary.contains("min trade")
        || summary.contains("notional")
        || summary.contains("tick")
        || summary.contains("insufficient")
        || summary.contains("margin")
        || summary.contains("balance")
        || summary.contains("reduce only")
        || summary.contains("reduce-only")
        || summary.contains("open interest")
        || summary.contains("oracle")
        || summary.contains("delist")
        || summary.contains("max position")
    {
        return TwapExchangeErrorAction::Terminal;
    }

    TwapExchangeErrorAction::ConsumeSlice
}

fn twap_terminal_cancel_error(summary: &str) -> bool {
    let summary = summary.to_ascii_lowercase();
    summary.contains("filled")
        || summary.contains("canceled")
        || summary.contains("cancelled")
        || summary.contains("cancled")
        || summary.contains("never placed")
        || summary.contains("not found")
        || summary.contains("does not exist")
        || summary.contains("no open order")
        || summary.contains("no longer open")
}

fn twap_cancel_target_matches(
    pending_oid: Option<u64>,
    pending_cloid: Option<&str>,
    oid: Option<u64>,
    cloid: Option<&str>,
) -> bool {
    oid.is_some() && pending_oid == oid
        || cloid.is_some() && pending_cloid == cloid
        || pending_oid.is_none() && oid.is_none() && pending_cloid == cloid
}

fn twap_child_matches_cancel_target(
    child: &TwapChildOrder,
    oid: Option<u64>,
    cloid: Option<&str>,
) -> bool {
    oid.is_some() && child.oid == oid || cloid.is_some() && child.cloid.as_deref() == cloid
}

fn twap_cancel_label(oid: Option<u64>, cloid: Option<&str>) -> String {
    match (oid, cloid) {
        (Some(oid), Some(cloid)) => format!("oid {oid} / {cloid}"),
        (Some(oid), None) => format!("oid {oid}"),
        (None, Some(cloid)) => cloid.to_string(),
        (None, None) => "unknown child".to_string(),
    }
}

fn twap_cancel_child_task(
    twap_id: u64,
    key: String,
    asset: u32,
    oid: Option<u64>,
    cloid: Option<String>,
) -> Task<Message> {
    if key.trim().is_empty() {
        return Task::perform(
            async { Err("original agent key unavailable".to_string()) },
            move |result| Message::TwapUnexpectedCancelResult {
                twap_id,
                oid,
                cloid: cloid.clone(),
                result: Box::new(result),
            },
        );
    }

    if let Some(cloid) = cloid {
        let request_cloid = cloid.clone();
        return Task::perform(
            cancel_order_by_cloid(key.into(), asset, request_cloid),
            move |result| Message::TwapUnexpectedCancelResult {
                twap_id,
                oid: None,
                cloid: Some(cloid.clone()),
                result: Box::new(result),
            },
        );
    }

    let Some(oid) = oid else {
        return Task::none();
    };
    Task::perform(cancel_order(key.into(), asset, oid), move |result| {
        Message::TwapUnexpectedCancelResult {
            twap_id,
            oid: Some(oid),
            cloid: None,
            result: Box::new(result),
        }
    })
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
    use super::{
        TwapAccountRefresh, TwapExchangeErrorAction, classify_twap_exchange_error,
        twap_cancel_target_matches, twap_ioc_limit_price, twap_place_result_refresh_policy,
    };
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::OrderStatusResult;
    use crate::app_state::TradingTerminal;
    use crate::signing::ExchangeResponse;
    use crate::twap_state::{
        TWAP_RECONCILIATION_TIMEOUT, TwapChildOrder, TwapChildStatus, TwapOrder, TwapPauseReason,
        TwapStatus,
    };
    use std::time::{Duration, Instant};

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

    fn filled_status(cloid: &str, oid: u64) -> OrderStatusResult {
        OrderStatusResult {
            status: "filled".to_string(),
            oid: Some(oid),
            cloid: Some(cloid.to_string()),
            raw_summary: format!("filled oid={oid} cloid={cloid}"),
        }
    }

    fn test_twap(id: u64, cloid: &str, now: Instant) -> TwapOrder {
        let mut twap = TwapOrder::new(
            id,
            "BTC".to_string(),
            "BTC".to_string(),
            "0xabc".to_string(),
            "test-agent-key".to_string().into(),
            true,
            1.0,
            0,
            3,
            false,
            false,
            90.0,
            110.0,
            false,
            Duration::from_secs(300),
            2,
            now,
            1_000,
        );
        twap.status = TwapStatus::Paused;
        twap.pause_reason = Some(TwapPauseReason::StatusUnknown);
        twap.status_check_cloid = Some(cloid.to_string());
        twap.child_orders.push(TwapChildOrder {
            index: 1,
            requested_at: now,
            planned_size: 0.5,
            limit_price: 100.0,
            oid: None,
            cloid: Some(cloid.to_string()),
            status: TwapChildStatus::StatusUnknown,
            exchange_summary: "status unknown".to_string(),
            filled_size: 0.0,
            avg_price: None,
            fee: 0.0,
            retry_count: 0,
        });
        twap
    }

    fn empty_account_data() -> AccountData {
        AccountData {
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
            fetched_at_ms: TradingTerminal::now_ms(),
        }
    }

    #[test]
    fn filled_status_check_arms_reconciliation_deadline() {
        let now = Instant::now();
        let cloid = "0x1234567890abcdef1234567890abcdef";
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.twap_orders.insert(1, test_twap(1, cloid, now));

        let _task = terminal.handle_twap_order_status_result(
            1,
            cloid.to_string(),
            Ok(filled_status(cloid, 42)),
        );

        let twap = terminal
            .twap_orders
            .get(&1)
            .expect("twap should remain active");
        assert_eq!(twap.status_check_cloid.as_deref(), Some(cloid));
        assert_eq!(
            twap.child_orders[0].status,
            TwapChildStatus::AwaitingReconciliation
        );
        let deadline = twap
            .reconciliation_deadline
            .expect("exchange-filled child must arm reconciliation watchdog");
        assert!(deadline > now);
        assert!(deadline <= Instant::now() + TWAP_RECONCILIATION_TIMEOUT);
    }

    #[test]
    fn reconciliation_timeout_fails_closed_when_account_fills_never_catch_up() {
        let now = Instant::now();
        let cloid = "0x1234567890abcdef1234567890abcdef";
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.account_data = Some(empty_account_data());
        let mut twap = test_twap(1, cloid, now);
        twap.child_orders[0].status = TwapChildStatus::AwaitingReconciliation;
        twap.status_check_cloid = Some(cloid.to_string());
        twap.reconciliation_deadline = Some(now);
        terminal.twap_orders.insert(1, twap);

        terminal.reconcile_twap_fills_from_account();

        let twap = terminal
            .twap_orders
            .get(&1)
            .expect("timed-out twap should remain inspectable");
        assert_eq!(twap.status, TwapStatus::Error);
        assert_eq!(twap.status_check_cloid, None);
        assert_eq!(twap.reconciliation_deadline, None);
        assert!(
            twap.events
                .iter()
                .any(|event| event.is_error && event.message.contains("Could not reconcile slice")),
            "timeout should leave an actionable error event"
        );
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

    #[test]
    fn advanced_exchange_requests_pause_while_account_reconciliation_is_loading() {
        let now = Instant::now();
        let mut terminal = TradingTerminal::boot().0;
        assert!(terminal.can_send_advanced_exchange_request(now));

        terminal.account_loading = true;

        assert!(!terminal.can_send_advanced_exchange_request(now));

        terminal.account_loading = false;
        terminal.account_reconciliation_required = true;

        assert!(!terminal.can_send_advanced_exchange_request(now));
    }

    #[test]
    fn twap_exchange_error_classification_separates_retryable_and_terminal_errors() {
        assert_eq!(
            classify_twap_exchange_error("Error: 429 Too Many Requests"),
            TwapExchangeErrorAction::Retry(TwapPauseReason::RateLimited)
        );
        assert_eq!(
            classify_twap_exchange_error("Error: Order must have minimum value of $10"),
            TwapExchangeErrorAction::Terminal
        );
        assert_eq!(
            classify_twap_exchange_error("Error: Order could not immediately match"),
            TwapExchangeErrorAction::ConsumeSlice
        );
    }

    #[test]
    fn twap_cancel_target_matches_by_oid_or_cloid() {
        assert!(twap_cancel_target_matches(
            Some(42),
            Some("0xabc"),
            Some(42),
            None
        ));
        assert!(twap_cancel_target_matches(
            None,
            Some("0xabc"),
            None,
            Some("0xabc")
        ));
        assert!(!twap_cancel_target_matches(
            Some(42),
            Some("0xabc"),
            Some(43),
            Some("0xdef")
        ));
    }
}
