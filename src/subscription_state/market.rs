use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::{
    ws_asset_ctx_stream_keyed, ws_book_stream_keyed, ws_hydromancer_asset_ctx_stream_keyed,
    ws_hydromancer_book_stream_keyed,
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
                    subs.push(
                        Subscription::run_with(
                            (api_key, ob.id, symbol.clone(), sigfigs),
                            ws_hydromancer_book_stream_keyed,
                        )
                        .map(|(id, coin, sigfigs, book)| {
                            Message::WsBookUpdate {
                                id,
                                coin,
                                sigfigs,
                                book,
                            }
                        }),
                    );
                } else {
                    subs.push(
                        Subscription::run_with(
                            (ob.id, symbol.clone(), sigfigs),
                            ws_book_stream_keyed,
                        )
                        .map(|(id, coin, sigfigs, book)| {
                            Message::WsBookUpdate {
                                id,
                                coin,
                                sigfigs,
                                book,
                            }
                        }),
                    );
                }
            }

            if streams.asset_ctx {
                if let Some(api_key) = self.hydromancer_read_provider_key() {
                    subs.push(
                        Subscription::run_with(
                            (api_key, ob.id, symbol.clone()),
                            ws_hydromancer_asset_ctx_stream_keyed,
                        )
                        .map(|(id, _symbol, ctx)| Message::OrderBookWsAssetCtxUpdate(id, ctx)),
                    );
                } else {
                    subs.push(
                        Subscription::run_with((ob.id, symbol.clone()), ws_asset_ctx_stream_keyed)
                            .map(|(id, _symbol, ctx)| Message::OrderBookWsAssetCtxUpdate(id, ctx)),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
