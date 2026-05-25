use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::ws_spaghetti_candle_stream;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Spaghetti Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_spaghetti_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
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
                            ws_spaghetti_candle_stream,
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
}
