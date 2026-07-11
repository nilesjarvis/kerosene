use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{
    HydromancerStreamKey, KeyedBookStreamEvent, ws_book_stream_keyed_events,
    ws_hydromancer_book_stream_keyed_events,
};

use iced::Subscription;

use super::source_context_for_stream_event;

// ---------------------------------------------------------------------------
// TWAP Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_twap_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if self
            .twap_orders
            .values()
            .any(|twap| twap.needs_timer_tick())
        {
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
            let source_context = self.market_data_source_context();
            if let Some(api_key) = self.hydromancer_read_provider_key() {
                let hydromancer_key_generation = self.hydromancer_key_generation;
                let stream_key =
                    HydromancerStreamKey::from_zeroizing(api_key, hydromancer_key_generation);
                subs.push(
                    Subscription::run_with(
                        (stream_key, twap.id, twap.coin.clone(), sigfigs),
                        ws_hydromancer_book_stream_keyed_events,
                    )
                    .with(source_context)
                    .map(twap_book_stream_event_message),
                );
            } else {
                subs.push(
                    Subscription::run_with(
                        (twap.id, twap.coin.clone(), sigfigs),
                        ws_book_stream_keyed_events,
                    )
                    .with(source_context)
                    .map(twap_book_stream_event_message),
                );
            }
        }
    }
}

fn twap_book_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedBookStreamEvent,
    ),
) -> Message {
    match event {
        KeyedBookStreamEvent::Item(twap_id, coin, sigfigs, hydromancer_key_generation, book) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::TwapBookUpdate {
                twap_id,
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
            Message::TwapBookLagged {
                twap_id: id,
                coin: coin.into(),
                sigfigs,
                source_context,
                skipped,
            }
        }
    }
}
