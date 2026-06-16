use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::{
    HydromancerStreamKey, KeyedAssetContextStreamEvent, KeyedBookStreamEvent,
    ws_asset_ctx_stream_keyed, ws_book_stream_keyed_events, ws_hydromancer_asset_ctx_stream_keyed,
    ws_hydromancer_book_stream_keyed_events,
};
use iced::Subscription;

mod chart;
mod chase;
mod order_book;
mod positioning_info;
mod spaghetti;
mod twap;
use order_book::*;

// ---------------------------------------------------------------------------
// Market Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::LiveWatchlist(_)))
        {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(15))
                    .map(|_| Message::LiveWatchlistRefreshTick),
            );
        }

        if self.ticker_tape_enabled && !self.favourite_symbols.is_empty() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60 * 5))
                    .map(|_| Message::TickerTapeRefreshTick),
            );
        }

        self.push_chart_market_subscriptions(subs);
        self.push_spaghetti_market_subscriptions(subs);
        self.push_order_book_subscriptions(subs);
        self.push_positioning_info_market_subscriptions(subs);
        self.push_chase_market_subscriptions(subs);
        self.push_twap_market_subscriptions(subs);
    }

    fn push_order_book_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let source_context = self.market_data_source_context();
        for ob in self.order_books.values() {
            let symbol = match &ob.mode {
                OrderBookSymbolMode::Active => self.active_symbol.clone(),
                OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
            };
            let streams = order_book_market_streams_for_symbol(
                &symbol,
                self.symbol_key_is_hidden(&symbol),
                self.is_outcome_coin(&symbol),
            );
            if streams.l2_book {
                let sigfigs = self.canonical_l2_book_sigfigs(&symbol);
                if let Some(api_key) = self.hydromancer_read_provider_key() {
                    let hydromancer_key_generation = self.hydromancer_key_generation;
                    let stream_key =
                        HydromancerStreamKey::from_zeroizing(api_key, hydromancer_key_generation);
                    subs.push(
                        Subscription::run_with(
                            (stream_key, ob.id, symbol.clone(), sigfigs),
                            ws_hydromancer_book_stream_keyed_events,
                        )
                        .with(source_context)
                        .map(order_book_stream_event_message),
                    );
                } else {
                    subs.push(
                        Subscription::run_with(
                            (ob.id, symbol.clone(), sigfigs),
                            ws_book_stream_keyed_events,
                        )
                        .with(source_context)
                        .map(order_book_stream_event_message),
                    );
                }
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

fn order_book_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedBookStreamEvent,
    ),
) -> Message {
    match event {
        KeyedBookStreamEvent::Item(id, coin, sigfigs, hydromancer_key_generation, book) => {
            debug_assert_eq!(
                source_context.hydromancer_key_generation,
                hydromancer_key_generation
            );
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
            debug_assert_eq!(
                source_context.hydromancer_key_generation,
                hydromancer_key_generation
            );
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

fn order_book_asset_ctx_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedAssetContextStreamEvent,
    ),
) -> Message {
    match event {
        KeyedAssetContextStreamEvent::Item(id, symbol, hydromancer_key_generation, ctx) => {
            debug_assert_eq!(
                source_context.hydromancer_key_generation,
                hydromancer_key_generation
            );
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
            debug_assert_eq!(
                source_context.hydromancer_key_generation,
                hydromancer_key_generation
            );
            Message::OrderBookWsAssetCtxLagged {
                id,
                coin: symbol,
                source_context,
                skipped,
            }
        }
    }
}

#[cfg(test)]
mod tests;
