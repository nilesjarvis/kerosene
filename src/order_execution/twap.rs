use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::twap_state::{TwapBookSnapshot, TwapEventKind, TwapPauseReason, TwapStatus};
use iced::Task;
use std::time::Instant;

mod cancel;
mod details;
mod execution;
mod fills;
mod helpers;
mod slice_result;
mod start;
mod status;

#[cfg(test)]
use self::helpers::{
    TwapAccountRefresh, TwapExchangeErrorAction, classify_twap_exchange_error,
    twap_cancel_target_matches, twap_ioc_limit_price, twap_place_result_refresh_policy,
};

// ---------------------------------------------------------------------------
// TWAP Advanced Orders
// ---------------------------------------------------------------------------

impl TradingTerminal {
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
        if self.symbol_key_is_hidden(&coin) {
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
        if self.expire_twap_reconciliation_timeouts(now) {
            return Task::none();
        }
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
}

#[cfg(test)]
mod tests;
