use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::helpers;
use crate::market_state::OrderBookSymbolMode;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::ws::{self, ws_asset_ctx_stream_keyed, ws_book_stream_keyed, ws_candle_stream_keyed};
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
        self.push_chase_market_subscriptions(subs);
        self.push_twap_market_subscriptions(subs);
    }

    fn push_chart_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        for instance in self.charts.values() {
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && !instance.symbol.is_empty()
                && !self.is_ticker_muted(&instance.symbol)
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
                && !self.is_ticker_muted(&instance.symbol)
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
                    && !self.is_ticker_muted(&series.symbol)
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
            if !symbol.is_empty()
                && !self.is_ticker_muted(&symbol)
                && !self.is_outcome_coin(&symbol)
            {
                let mid = ob.book.mid_price();
                let sigfigs = helpers::compute_sigfigs(ob.tick_size, mid);
                subs.push(
                    Subscription::run_with((ob.id, symbol.clone(), sigfigs), ws_book_stream_keyed)
                        .map(|(id, coin, book)| Message::WsBookUpdate(id, coin, book)),
                );

                subs.push(
                    Subscription::run_with((ob.id, symbol.clone()), ws_asset_ctx_stream_keyed)
                        .map(|(id, _symbol, ctx)| Message::OrderBookWsAssetCtxUpdate(id, ctx)),
                );
            }
        }
    }

    fn push_chase_market_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self
            .chase_orders
            .values()
            .any(|chase| chase.pending_best_price.is_some())
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
                || self.is_ticker_muted(&chase.coin)
                || self.is_outcome_coin(&chase.coin)
            {
                continue;
            }
            let sigfigs = self.chase_book_fetch_sigfigs(&chase.coin);
            subs.push(
                Subscription::run_with(
                    (chase.id, chase.coin.clone(), sigfigs),
                    ws_book_stream_keyed,
                )
                .map(|(chase_id, coin, book)| Message::ChaseBookUpdate {
                    chase_id,
                    coin,
                    book,
                }),
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
                || self.is_ticker_muted(&twap.coin)
                || self.is_outcome_coin(&twap.coin)
            {
                continue;
            }
            let sigfigs = self.chase_book_fetch_sigfigs(&twap.coin);
            subs.push(
                Subscription::run_with((twap.id, twap.coin.clone(), sigfigs), ws_book_stream_keyed)
                    .map(|(twap_id, coin, book)| Message::TwapBookUpdate {
                        twap_id,
                        coin,
                        book,
                    }),
            );
        }
    }
}
