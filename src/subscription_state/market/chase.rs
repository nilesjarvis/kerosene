use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};
use crate::ws::KeyedBookStreamEvent;

use iced::Subscription;

use super::source_context_for_stream_event;

// ---------------------------------------------------------------------------
// Chase Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_chase_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if self.chase_orders.values().any(|chase| {
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
        }) {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(250))
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
