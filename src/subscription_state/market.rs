use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::{
    self, ws_asset_ctx_stream_keyed, ws_asset_ctx_stream_symbol, ws_book_stream_keyed,
    ws_candle_stream_keyed,
};
use iced::Subscription;

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

        self.push_chart_market_subscriptions(subs);
        self.push_spaghetti_market_subscriptions(subs);
        self.push_order_book_subscriptions(subs);
        self.push_positioning_info_market_subscriptions(subs);
        self.push_chase_market_subscriptions(subs);
        self.push_twap_market_subscriptions(subs);
    }

    fn push_chart_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        for instance in self.charts.values() {
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && !instance.symbol.is_empty()
                && !self.symbol_key_is_hidden(&instance.symbol)
            {
                subs.push(
                    Subscription::run_with(
                        (
                            instance.id,
                            instance.symbol.clone(),
                            instance.interval.api_str().to_string(),
                        ),
                        ws_candle_stream_keyed,
                    )
                    .map(|(id, symbol, interval, candle)| {
                        Message::ChartWsCandleUpdate(id, symbol, interval, candle)
                    }),
                );
            }
            if !instance.symbol.is_empty()
                && !self.symbol_key_is_hidden(&instance.symbol)
                && !self.is_outcome_coin(&instance.symbol)
            {
                subs.push(
                    Subscription::run_with(
                        (instance.id, instance.symbol.clone()),
                        ws_asset_ctx_stream_keyed,
                    )
                    .map(|(id, symbol, ctx)| Message::ChartWsAssetCtxUpdate(id, symbol, ctx)),
                );
            }
        }
    }

    fn push_spaghetti_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let now_ms = Self::now_ms();
        for inst in self.spaghetti_charts.values() {
            let (ws_tf, _) = Self::spaghetti_fetch_plan(
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                now_ms,
            );
            for series in &inst.canvas.series {
                if series.loaded
                    && !series.symbol.is_empty()
                    && !self.symbol_key_is_hidden(&series.symbol)
                {
                    subs.push(
                        Subscription::run_with(
                            (
                                10000 + inst.id,
                                series.symbol.clone(),
                                ws_tf.api_str().to_string(),
                            ),
                            ws::ws_spaghetti_candle_stream,
                        )
                        .map(|(sid, coin, candle)| {
                            Message::SpaghettiWsCandleUpdate(
                                sid.saturating_sub(10000),
                                coin,
                                candle,
                            )
                        }),
                    );
                }
            }
        }
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
                subs.push(
                    Subscription::run_with((ob.id, symbol.clone(), sigfigs), ws_book_stream_keyed)
                        .map(|(id, coin, sigfigs, book)| Message::WsBookUpdate {
                            id,
                            coin,
                            sigfigs,
                            book,
                        }),
                );
            }

            if streams.asset_ctx {
                subs.push(
                    Subscription::run_with((ob.id, symbol.clone()), ws_asset_ctx_stream_keyed)
                        .map(|(id, _symbol, ctx)| Message::OrderBookWsAssetCtxUpdate(id, ctx)),
                );
            }
        }
    }

    fn push_positioning_info_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self.hyperdash_api_key.trim().is_empty() {
            return;
        }

        let mut symbols = Vec::new();
        for (_, kind) in self.panes.iter() {
            let PaneKind::PositioningInfo(id) = kind else {
                continue;
            };
            let Some(instance) = self.positioning_infos.get(id) else {
                continue;
            };
            if instance.symbol.is_empty()
                || self.symbol_key_is_hidden(&instance.symbol)
                || self.hyperdash_coin_for_symbol(&instance.symbol).is_none()
                || symbols.iter().any(|symbol| symbol == &instance.symbol)
            {
                continue;
            }
            symbols.push(instance.symbol.clone());
        }

        for symbol in symbols {
            subs.push(
                Subscription::run_with((symbol.clone(),), ws_asset_ctx_stream_symbol)
                    .map(|(symbol, ctx)| Message::PositioningInfoWsAssetCtxUpdate(symbol, ctx)),
            );
        }
    }

    fn push_chase_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self
            .chase_orders
            .values()
            .any(|chase| chase.pending_best_price.is_some() || chase.pending_size_correction)
        {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(250))
                    .map(|_| Message::ChaseRepriceTick),
            );
        }

        for chase in self.chase_orders.values() {
            if chase.coin.is_empty()
                || chase.current_oid.is_none()
                || chase.stop_requested
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

    fn push_twap_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
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
            subs.push(
                Subscription::run_with((twap.id, twap.coin.clone(), sigfigs), ws_book_stream_keyed)
                    .map(|(twap_id, coin, _sigfigs, book)| Message::TwapBookUpdate {
                        twap_id,
                        coin,
                        book,
                    }),
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderBookMarketStreams {
    l2_book: bool,
    asset_ctx: bool,
}

fn order_book_market_streams_for_symbol(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_order_books_subscribe_to_l2_without_asset_ctx() {
        assert_eq!(
            order_book_market_streams_for_symbol("#650", false, true),
            OrderBookMarketStreams {
                l2_book: true,
                asset_ctx: false,
            }
        );
    }

    #[test]
    fn non_outcome_order_books_subscribe_to_l2_and_asset_ctx() {
        assert_eq!(
            order_book_market_streams_for_symbol("BTC", false, false),
            OrderBookMarketStreams {
                l2_book: true,
                asset_ctx: true,
            }
        );
    }

    #[test]
    fn hidden_or_empty_order_books_do_not_subscribe() {
        let disabled = OrderBookMarketStreams {
            l2_book: false,
            asset_ctx: false,
        };

        assert_eq!(
            order_book_market_streams_for_symbol("", false, false),
            disabled
        );
        assert_eq!(
            order_book_market_streams_for_symbol("BTC", true, false),
            disabled
        );
    }
}
