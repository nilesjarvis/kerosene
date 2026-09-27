use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::KeyedBookStreamEvent;

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
            subs.push(
                self.market_book_subscription(twap.id, &twap.coin)
                    .map(twap_book_stream_event_message),
            );
        }
    }
}

pub(super) fn twap_book_stream_event_message(
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
            Message::TwapBookLagged {
                twap_id: id,
                coin,
                sigfigs,
                source_context,
                skipped,
            }
        }
    }
}
