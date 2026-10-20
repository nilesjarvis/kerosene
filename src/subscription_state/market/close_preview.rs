use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::read_data_provider::MarketDataSourceContext;
use crate::ws::KeyedBookStreamEvent;
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_position_close_preview_subscription(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        let preview = &self.position_close_preview;
        let Some(coin) = preview.coin.as_deref() else {
            return;
        };
        if !self.position_close_preview_is_visible(coin) {
            return;
        }
        // Unaggregated prices are essential: chart/display tick buckets cannot
        // provide a trustworthy execution VWAP.
        subs.push(
            self.market_book_subscription_at_precision(preview.generation, coin, (None, None))
                .map(position_close_book_message),
        );
    }
}

pub(super) fn position_close_book_message(
    (source_context, event): (MarketDataSourceContext, KeyedBookStreamEvent),
) -> Message {
    let (generation, coin, key_generation, book) = match event {
        KeyedBookStreamEvent::Item(id, coin, _, key_generation, book) => {
            (id, coin, key_generation, Some(book))
        }
        KeyedBookStreamEvent::Lagged {
            id,
            coin,
            hydromancer_key_generation,
            ..
        } => (id, coin, hydromancer_key_generation, None),
    };
    Message::PositionCloseBook {
        generation,
        coin,
        source_context: super::source_context_for_stream_event(source_context, key_generation),
        book,
    }
}
