use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};
use crate::ws::{
    HydromancerStreamKey, KeyedBookStreamEvent, ws_book_stream_keyed_events,
    ws_hydromancer_book_stream_keyed_events,
};

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
            let sigfigs = self.canonical_l2_book_sigfigs(&chase.coin);
            let source_context = self.market_data_source_context();
            if let Some(api_key) = self.hydromancer_read_provider_key() {
                let hydromancer_key_generation = self.hydromancer_key_generation;
                let stream_key =
                    HydromancerStreamKey::from_zeroizing(api_key, hydromancer_key_generation);
                subs.push(
                    Subscription::run_with(
                        (stream_key, chase.id, chase.coin.clone(), sigfigs),
                        ws_hydromancer_book_stream_keyed_events,
                    )
                    .with(source_context)
                    .map(chase_book_stream_event_message),
                );
            } else {
                subs.push(
                    Subscription::run_with(
                        (chase.id, chase.coin.clone(), sigfigs),
                        ws_book_stream_keyed_events,
                    )
                    .with(source_context)
                    .map(chase_book_stream_event_message),
                );
            }
        }
    }
}

fn chase_book_stream_event_message(
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
                coin: coin.into(),
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
                coin: coin.into(),
                sigfigs,
                source_context,
                skipped,
            }
        }
    }
}
