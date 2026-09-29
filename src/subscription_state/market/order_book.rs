use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::ws::{
    HydromancerStreamKey, KeyedAssetContextStreamEvent, KeyedBookStreamEvent,
    ws_asset_ctx_stream_keyed, ws_hydromancer_asset_ctx_stream_keyed,
};
use iced::Subscription;

use super::source_context_for_stream_event;

// ---------------------------------------------------------------------------
// Order Book Market Streams
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrderBookMarketStreams {
    pub(super) l2_book: bool,
    pub(super) asset_ctx: bool,
}

pub(super) fn order_book_market_streams_for_symbol(
    symbol: &str,
    hidden: bool,
    outcome: bool,
) -> OrderBookMarketStreams {
    let market_data_enabled = !symbol.is_empty() && !hidden;
    OrderBookMarketStreams {
        l2_book: market_data_enabled,
        asset_ctx: market_data_enabled && !outcome,
    }
}

impl TradingTerminal {
    pub(super) fn push_order_book_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let source_context = self.market_data_source_context();
        for ob in self.order_books.values() {
            let symbol = match &ob.mode {
                OrderBookSymbolMode::Active => &self.active_symbol,
                OrderBookSymbolMode::Fixed(symbol) => symbol,
            };
            let streams = order_book_market_streams_for_symbol(
                symbol,
                self.symbol_key_is_hidden(symbol),
                self.is_outcome_coin(symbol),
            );
            if streams.l2_book {
                subs.push(
                    self.market_book_subscription(ob.id, symbol)
                        .map(order_book_stream_event_message),
                );
            }

            if streams.asset_ctx {
                if let Some(api_key) = self.hydromancer_read_provider_key() {
                    let hydromancer_key_generation = self.hydromancer_key_generation;
                    let stream_key =
                        HydromancerStreamKey::from_zeroizing(api_key, hydromancer_key_generation);
                    subs.push(
                        Subscription::run_with(
                            (stream_key, ob.id, symbol.clone()),
                            ws_hydromancer_asset_ctx_stream_keyed,
                        )
                        .with(source_context)
                        .map(order_book_asset_ctx_stream_event_message),
                    );
                } else {
                    subs.push(
                        Subscription::run_with((ob.id, symbol.clone()), ws_asset_ctx_stream_keyed)
                            .with(source_context)
                            .map(order_book_asset_ctx_stream_event_message),
                    );
                }
            }
        }
    }
}

pub(super) fn order_book_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedBookStreamEvent,
    ),
) -> Message {
    match event {
        KeyedBookStreamEvent::Item(id, coin, sigfigs, hydromancer_key_generation, book) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::WsBookUpdate {
                id,
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
            Message::OrderBookWsBookLagged {
                id,
                coin,
                sigfigs,
                source_context,
                skipped,
            }
        }
    }
}

pub(super) fn order_book_asset_ctx_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedAssetContextStreamEvent,
    ),
) -> Message {
    match event {
        KeyedAssetContextStreamEvent::Item(id, symbol, hydromancer_key_generation, ctx) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::OrderBookWsAssetCtxUpdate {
                id,
                coin: symbol,
                source_context,
                ctx: *ctx,
            }
        }
        KeyedAssetContextStreamEvent::Lagged {
            id,
            symbol,
            hydromancer_key_generation,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::OrderBookWsAssetCtxLagged {
                id,
                coin: symbol,
                source_context,
                skipped,
            }
        }
    }
}
