use self::planning::validate_twap_planned_slice;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use crate::order_execution::{OrderSurface, PreparedExchangeOrder, place_order_task};
use crate::signing::{ExchangeOrderKind, float_to_wire};
use crate::twap_state::{
    TWAP_BOOK_STALE_AFTER, TwapChildOrder, TwapChildStatus, TwapEventKind, TwapPauseReason,
    TwapPendingOp, TwapPendingSlice, TwapStatus, quantize_twap_slice_size, twap_child_cloid,
};

use iced::Task;
use std::time::Instant;

mod lifecycle;
mod planning;
mod skip;

// ---------------------------------------------------------------------------
// TWAP Slice Execution
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn execute_due_twap_slice(&mut self, twap_id: u64, now: Instant) -> Task<Message> {
        if self.expire_twap_if_deadline_passed(twap_id, now) {
            return Task::none();
        }
        if let Some(twap) = self.twap_orders.get(&twap_id)
            && !self.connected_order_account_matches(&twap.account_address)
        {
            return self.stop_twap_with_reason(
                twap_id,
                "TWAP stopped: account changed before slice",
                true,
            );
        }
        // Defense in depth against hidden symbols. The mute handler already
        // stops matching TWAPs (see
        // `update_muted_ticker_preferences`), but this catches:
        //   (a) a mute or market-universe change that landed between the
        //       schedule tick and this execute call;
        //   (b) any future code path that adds a TWAP without going through
        //       the hidden-symbol eviction pass.
        // Without it, a freshly-hidden symbol could fire one more slice off
        // its cached `latest_book` before the stop landed.
        if let Some(twap) = self.twap_orders.get(&twap_id)
            && self.symbol_key_is_hidden(&twap.coin)
        {
            return self.stop_twap_with_reason(twap_id, "TWAP stopped: ticker was hidden", false);
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

        if is_spot {
            let symbol_key = self
                .twap_orders
                .get(&twap_id)
                .map(|twap| twap.coin.as_str())
                .unwrap_or_default();
            if !self.twap_spot_symbol_identity_is_current(twap_id, symbol_key) {
                return self.stop_twap_with_reason(
                    twap_id,
                    "TWAP stopped: spot market identity changed",
                    true,
                );
            }
        }

        let spot_quote_error = if is_spot {
            self.twap_orders.get(&twap_id).and_then(|twap| {
                self.validate_spot_quantity_denomination(&twap.coin, false)
                    .err()
            })
        } else {
            None
        };
        if let Some(message) = spot_quote_error {
            return self.stop_twap_with_reason(twap_id, message, true);
        }

        if is_spot && self.spot_metadata_degraded {
            if let Some(twap) = self.twap_orders.get_mut(&twap_id)
                && twap.pause_reason != Some(TwapPauseReason::SpotMetadataUnverified)
            {
                twap.pause(
                    TwapPauseReason::SpotMetadataUnverified,
                    None,
                    "TWAP paused: spot metadata has not been verified".to_string(),
                    true,
                );
                self.order_status = Some((
                    "TWAP paused: spot metadata has not been verified".to_string(),
                    true,
                ));
            }
            return Task::none();
        }
        if is_spot
            && let Some(twap) = self.twap_orders.get_mut(&twap_id)
            && twap.pause_reason == Some(TwapPauseReason::SpotMetadataUnverified)
        {
            twap.clear_pause();
            twap.status = TwapStatus::Running;
            twap.push_event(
                TwapEventKind::Reconciled,
                "TWAP resumed: spot metadata verified".to_string(),
                false,
            );
        }

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

        let limit_price = match validate_twap_planned_slice(
            &book,
            is_buy,
            planned_size,
            min_price,
            max_price,
            sz_decimals,
            is_spot,
        ) {
            Ok(limit_price) => limit_price,
            Err(skip) => {
                self.record_twap_slice_skip(
                    twap_id,
                    now,
                    retry_slice.as_ref(),
                    skip.kind,
                    skip.message,
                    skip.is_error,
                );
                return Task::none();
            }
        };

        let Some(twap) = self.twap_orders.get_mut(&twap_id) else {
            return Task::none();
        };
        let key = twap.agent_key.clone_for_task();
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
            if let Some(child) = twap.child_order_mut(slice.index) {
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
        let account_address = twap.account_address.clone();
        let market_type = if twap.is_spot {
            MarketType::Spot
        } else {
            MarketType::Perp
        };
        self.last_advanced_exchange_request_at = Some(now);

        let prepared = PreparedExchangeOrder {
            surface: OrderSurface::Twap,
            symbol_key: twap.coin.clone(),
            asset,
            is_buy,
            price: float_to_wire(limit_price),
            size: float_to_wire(planned_size),
            order_kind: ExchangeOrderKind::LimitIoc,
            reduce_only,
            market_type,
        };
        let request = prepared.place_request_with_existing_cloid(pending_slice.cloid);

        self.invalidate_spot_balances_after_exchange_dispatch(&account_address, market_type);
        place_order_task(key, request, move |result| Message::TwapSliceResult {
            twap_id,
            result: Box::new(result),
        })
    }
}
