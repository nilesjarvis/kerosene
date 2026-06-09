use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{ws_book_stream_keyed, ws_hydromancer_book_stream_keyed};

use iced::Subscription;

// ---------------------------------------------------------------------------
// TWAP Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_twap_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if self.twap_orders.values().any(|twap| {
            !twap.status.is_terminal() && !twap.stop_requested && twap.pending_op.is_none()
        }) {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::TwapTick),
            );
        }

        for twap in self.twap_orders.values() {
            if twap.coin.is_empty()
                || twap.status.is_terminal()
                || twap.stop_requested
                || self.symbol_key_is_hidden(&twap.coin)
                || self.is_outcome_coin(&twap.coin)
            {
                continue;
            }
            let sigfigs = self.canonical_l2_book_sigfigs(&twap.coin);
            if let Some(api_key) = self.hydromancer_read_provider_key() {
                subs.push(
                    Subscription::run_with(
                        (api_key, twap.id, twap.coin.clone(), sigfigs),
                        ws_hydromancer_book_stream_keyed,
                    )
                    .map(|(twap_id, coin, _sigfigs, book)| {
                        Message::TwapBookUpdate {
                            twap_id,
                            coin,
                            book,
                        }
                    }),
                );
            } else {
                subs.push(
                    Subscription::run_with(
                        (twap.id, twap.coin.clone(), sigfigs),
                        ws_book_stream_keyed,
                    )
                    .map(|(twap_id, coin, _sigfigs, book)| {
                        Message::TwapBookUpdate {
                            twap_id,
                            coin,
                            book,
                        }
                    }),
                );
            }
        }
    }
}
