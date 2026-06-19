use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::message::Message;
use crate::timeframe::Timeframe;
use crate::ws::{
    HydromancerStreamKey, KeyedAssetContextStreamEvent, KeyedCandleStreamEvent,
    ws_asset_ctx_stream_keyed, ws_candle_stream_keyed, ws_hydromancer_asset_ctx_stream_keyed,
    ws_hydromancer_candle_stream_keyed,
};

use super::source_context_for_stream_event;
use iced::Subscription;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Chart Market Streams
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::subscription_state::market) fn push_chart_market_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        let mut candle_streams: BTreeMap<(String, String), u64> = BTreeMap::new();
        let mut asset_ctx_streams: BTreeMap<String, u64> = BTreeMap::new();

        for instance in self.charts.values() {
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && !instance.symbol.is_empty()
                && instance.interval.uses_candle_backfill()
                && !self.symbol_key_is_hidden(&instance.symbol)
            {
                let key = (
                    instance.symbol.clone(),
                    instance.interval.api_str().to_string(),
                );
                candle_streams
                    .entry(key)
                    .and_modify(|id| *id = (*id).min(instance.id))
                    .or_insert(instance.id);
            }
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && let Some(symbol) = instance.secondary_symbol.as_ref()
                && instance.interval.uses_candle_backfill()
                && !self.symbol_key_is_hidden(symbol)
                && instance
                    .chart
                    .secondary_series
                    .as_ref()
                    .is_some_and(|series| !series.candles.is_empty())
            {
                let key = (symbol.clone(), instance.interval.api_str().to_string());
                candle_streams
                    .entry(key)
                    .and_modify(|id| *id = (*id).min(instance.id))
                    .or_insert(instance.id);
            }
            if !instance.symbol.is_empty() && !self.symbol_key_is_hidden(&instance.symbol) {
                asset_ctx_streams
                    .entry(instance.symbol.clone())
                    .and_modify(|id| *id = (*id).min(instance.id))
                    .or_insert(instance.id);
            }
        }

        let hydromancer_key_generation = self.hydromancer_key_generation;
        let hydromancer_key = (!self.hydromancer_api_key.trim().is_empty()).then(|| {
            HydromancerStreamKey::from_zeroizing(
                self.hydromancer_api_key_for_task(),
                hydromancer_key_generation,
            )
        });
        let hydromancer_read_provider_key = self.hydromancer_read_provider_key().map(|api_key| {
            HydromancerStreamKey::from_zeroizing(api_key, hydromancer_key_generation)
        });
        let source_context = self.market_data_source_context();
        let hydromancer_keyed_source_context = self.hydromancer_keyed_market_data_source_context();
        for ((symbol, interval), id) in candle_streams {
            let hydromancer_only_interval = interval == Timeframe::S1.api_str();
            let use_hydromancer_stream =
                hydromancer_only_interval || self.hydromancer_read_provider_enabled();
            if use_hydromancer_stream {
                let Some(api_key) = hydromancer_key.clone() else {
                    continue;
                };
                let stream_source_context = if hydromancer_only_interval {
                    hydromancer_keyed_source_context
                } else {
                    source_context
                };
                subs.push(
                    Subscription::run_with(
                        (api_key, id, symbol, interval),
                        ws_hydromancer_candle_stream_keyed,
                    )
                    .with(stream_source_context)
                    .map(chart_candle_stream_event_message),
                );
            } else {
                subs.push(
                    Subscription::run_with((id, symbol, interval), ws_candle_stream_keyed)
                        .with(source_context)
                        .map(chart_candle_stream_event_message),
                );
            }
        }
        for (symbol, id) in asset_ctx_streams {
            if let Some(api_key) = hydromancer_read_provider_key.clone() {
                subs.push(
                    Subscription::run_with(
                        (api_key, id, symbol),
                        ws_hydromancer_asset_ctx_stream_keyed,
                    )
                    .with(source_context)
                    .map(chart_asset_ctx_stream_event_message),
                );
            } else {
                subs.push(
                    Subscription::run_with((id, symbol), ws_asset_ctx_stream_keyed)
                        .with(source_context)
                        .map(chart_asset_ctx_stream_event_message),
                );
            }
        }
    }
}

fn chart_candle_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedCandleStreamEvent,
    ),
) -> Message {
    match event {
        KeyedCandleStreamEvent::Item(id, symbol, interval, hydromancer_key_generation, candle) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::ChartWsCandleUpdate(id, symbol, interval, source_context, candle)
        }
        KeyedCandleStreamEvent::Lagged {
            id,
            symbol,
            interval,
            hydromancer_key_generation,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::ChartWsCandleLagged(id, symbol, interval, source_context, skipped)
        }
    }
}

pub(super) fn chart_asset_ctx_stream_event_message(
    (source_context, event): (
        crate::read_data_provider::MarketDataSourceContext,
        KeyedAssetContextStreamEvent,
    ),
) -> Message {
    match event {
        KeyedAssetContextStreamEvent::Item(id, symbol, hydromancer_key_generation, ctx) => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::ChartWsAssetCtxUpdate(id, symbol, source_context, *ctx)
        }
        KeyedAssetContextStreamEvent::Lagged {
            id,
            symbol,
            hydromancer_key_generation,
            skipped,
        } => {
            let source_context =
                source_context_for_stream_event(source_context, hydromancer_key_generation);
            Message::ChartWsAssetCtxLagged(id, symbol, source_context, skipped)
        }
    }
}
