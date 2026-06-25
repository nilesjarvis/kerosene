use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{
    HydromancerStreamKey, KeyedBookStreamEvent, ws_hydromancer_book_stream_keyed_events,
};

use super::source_context_for_stream_event;
use iced::Subscription;

// ---------------------------------------------------------------------------
// Real-Time Position PnL Streams
// ---------------------------------------------------------------------------

const POSITION_PNL_BOOK_STREAM_ID: u64 = 0;

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_position_pnl_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        let symbols = self.hydromancer_realtime_position_pnl_symbols();
        if symbols.is_empty() {
            return;
        }

        let stream_key = HydromancerStreamKey::from_zeroizing(
            self.hydromancer_api_key_for_task(),
            self.hydromancer_key_generation,
        );
        let source_context = self.hydromancer_keyed_market_data_source_context();
        for symbol in symbols {
            let sigfigs = self.canonical_l2_book_sigfigs(&symbol);
            subs.push(
                Subscription::run_with(
                    (
                        stream_key.clone(),
                        POSITION_PNL_BOOK_STREAM_ID,
                        symbol,
                        sigfigs,
                    ),
                    ws_hydromancer_book_stream_keyed_events,
                )
                .with(source_context)
                .map(position_pnl_book_stream_event_message),
            );
        }
    }
}

pub(super) fn position_pnl_book_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedBookStreamEvent,
    ),
) -> Message {
    match event {
        KeyedBookStreamEvent::Item(_id, coin, sigfigs, hydromancer_key_generation, book) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::PositionPnlWsBookUpdate {
                coin,
                sigfigs,
                source_context,
                book,
            }
        }
        KeyedBookStreamEvent::Lagged {
            id: _,
            coin,
            sigfigs,
            hydromancer_key_generation,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::PositionPnlWsBookLagged {
                coin,
                sigfigs,
                source_context,
                skipped,
            }
        }
    }
}
