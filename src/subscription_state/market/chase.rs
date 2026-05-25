use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};
use crate::ws::ws_book_stream_keyed;

use iced::Subscription;

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
            subs.push(
                Subscription::run_with(
                    (chase.id, chase.coin.clone(), sigfigs),
                    ws_book_stream_keyed,
                )
                .map(
                    |(chase_id, coin, _sigfigs, book)| Message::ChaseBookUpdate {
                        chase_id,
                        coin,
                        book,
                    },
                ),
            );
        }
    }
}
