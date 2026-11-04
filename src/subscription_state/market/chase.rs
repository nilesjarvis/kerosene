use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{
    ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason, MIN_CHASE_REPRICE_INTERVAL,
};
use crate::twap_state::ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL;
use crate::ws::KeyedBookStreamEvent;

use futures::SinkExt;
use iced::Subscription;
use std::time::{Duration, Instant};

use super::source_context_for_stream_event;

// ---------------------------------------------------------------------------
// Chase Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_chase_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if let Some(deadline) = self.chase_reprice_wakeup_at() {
            subs.push(
                Subscription::run_with(deadline, chase_reprice_tick_stream)
                    .map(|_| Message::ChaseRepriceTick),
            );
        }

        for chase in self.chase_orders.values() {
            if chase.coin.is_empty()
                || chase.current_oid.is_none()
                || chase.lifecycle.is_stopping()
                || self.symbol_key_is_hidden(&chase.coin)
                || self.is_outcome_coin(&chase.coin)
            {
                continue;
            }
            subs.push(
                self.market_book_subscription(chase.id, &chase.coin)
                    .map(chase_book_stream_event_message),
            );
        }
    }

    pub(super) fn chase_reprice_wakeup_at(&self) -> Option<Instant> {
        self.chase_orders
            .values()
            .filter(|chase| {
                matches!(
                    chase.lifecycle,
                    ChaseLifecycle::Queued { .. }
                        | ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::Placement
                                | ChaseVerificationReason::Modify
                                | ChaseVerificationReason::MissingOrder
                        }
                        | ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::AwaitingPlace
                        }
                        | ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { .. }
                        }
                )
            })
            .map(|chase| {
                let per_order = chase
                    .last_reprice_at
                    .map_or(chase.started_at, |last| last + MIN_CHASE_REPRICE_INTERVAL);
                self.last_advanced_exchange_request_at
                    .map_or(per_order, |last| {
                        per_order.max(last + ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL)
                    })
            })
            .min()
    }
}

fn chase_reprice_tick_stream(
    deadline: &Instant,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = ()> + Send>> {
    let deadline = *deadline;
    Box::pin(iced::stream::channel(1, async move |mut output| {
        tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
        // The first wakeup is at eligibility, not the next polling boundary.
        // Keep a bounded fallback for account loading and no-action states.
        loop {
            if output.send(()).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }))
}

pub(super) fn chase_book_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedBookStreamEvent,
    ),
) -> Message {
    match event {
        KeyedBookStreamEvent::Item(chase_id, coin, sigfigs, hydromancer_key_generation, book) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::ChaseBookUpdate {
                chase_id,
                coin,
                sigfigs,
                source_context,
                book,
            }
        }
        KeyedBookStreamEvent::Lagged {
            id,
            coin,
            sigfigs,
            hydromancer_key_generation,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::ChaseBookLagged {
                chase_id: id,
                coin,
                sigfigs,
                source_context,
                skipped,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test(start_paused = true)]
    async fn chase_tick_wakes_at_deadline_and_keeps_a_bounded_fallback() {
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut stream = chase_reprice_tick_stream(&deadline);
        let mut next = Box::pin(stream.next());
        assert!(futures::poll!(&mut next).is_pending());
        tokio::time::advance(Duration::from_millis(990)).await;
        assert!(futures::poll!(&mut next).is_pending());
        tokio::time::advance(Duration::from_millis(11)).await;
        assert_eq!(next.await, Some(()));
        let mut next = Box::pin(stream.next());
        assert!(futures::poll!(&mut next).is_pending());
        tokio::time::advance(Duration::from_millis(240)).await;
        assert!(futures::poll!(&mut next).is_pending());
        tokio::time::advance(Duration::from_millis(11)).await;
        assert_eq!(next.await, Some(()));
    }
}
