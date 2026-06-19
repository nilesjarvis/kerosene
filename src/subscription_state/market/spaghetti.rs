use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiWsCandleContext;
use crate::ws::{
    HydromancerStreamKey, SpaghettiCandleStreamEvent, ws_hydromancer_spaghetti_candle_stream,
    ws_spaghetti_candle_stream,
};

use iced::Subscription;

use super::source_context_for_stream_event;

// ---------------------------------------------------------------------------
// Spaghetti Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_spaghetti_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        let now_ms = Self::now_ms();
        let hydromancer_key_generation = self.hydromancer_key_generation;
        let hydromancer_key = self.hydromancer_read_provider_key().map(|api_key| {
            HydromancerStreamKey::from_zeroizing(api_key, hydromancer_key_generation)
        });
        let source_context = self.market_data_source_context();
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
                    if let Some(api_key) = hydromancer_key.clone() {
                        subs.push(
                            Subscription::run_with(
                                (
                                    api_key,
                                    10000 + inst.id,
                                    series.symbol.clone(),
                                    ws_tf,
                                    inst.canvas.active_session,
                                    inst.session_granularity,
                                ),
                                ws_hydromancer_spaghetti_candle_stream,
                            )
                            .with(source_context)
                            .map(spaghetti_candle_stream_event_to_message),
                        );
                    } else {
                        subs.push(
                            Subscription::run_with(
                                (
                                    10000 + inst.id,
                                    series.symbol.clone(),
                                    ws_tf,
                                    inst.canvas.active_session,
                                    inst.session_granularity,
                                ),
                                ws_spaghetti_candle_stream,
                            )
                            .with(source_context)
                            .map(spaghetti_candle_stream_event_to_message),
                        );
                    }
                }
            }
        }
    }
}

fn spaghetti_candle_stream_event_to_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        SpaghettiCandleStreamEvent,
    ),
) -> Message {
    match event {
        SpaghettiCandleStreamEvent::Item {
            id,
            symbol,
            timeframe,
            hydromancer_key_generation,
            session,
            session_granularity,
            candle,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::SpaghettiWsCandleUpdate(
                SpaghettiWsCandleContext {
                    chart_id: id.saturating_sub(10000),
                    symbol,
                    timeframe,
                    source_context,
                    session,
                    session_granularity,
                },
                candle,
            )
        }
        SpaghettiCandleStreamEvent::Lagged {
            id,
            symbol,
            timeframe,
            hydromancer_key_generation,
            session,
            session_granularity,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::SpaghettiWsCandleLagged(
                SpaghettiWsCandleContext {
                    chart_id: id.saturating_sub(10000),
                    symbol,
                    timeframe,
                    source_context,
                    session,
                    session_granularity,
                },
                skipped,
            )
        }
    }
}
