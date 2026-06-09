use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::message::Message;
use crate::ws::{
    ws_asset_ctx_stream_keyed, ws_candle_stream_keyed, ws_hydromancer_asset_ctx_stream_keyed,
    ws_hydromancer_candle_stream_keyed,
};

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
            if !instance.symbol.is_empty() && !self.symbol_key_is_hidden(&instance.symbol) {
                asset_ctx_streams
                    .entry(instance.symbol.clone())
                    .and_modify(|id| *id = (*id).min(instance.id))
                    .or_insert(instance.id);
            }
        }

        let hydromancer_key = self.hydromancer_read_provider_key();
        for ((symbol, interval), id) in candle_streams {
            if let Some(api_key) = hydromancer_key.clone() {
                subs.push(
                    Subscription::run_with(
                        (api_key, id, symbol, interval),
                        ws_hydromancer_candle_stream_keyed,
                    )
                    .map(|(id, symbol, interval, candle)| {
                        Message::ChartWsCandleUpdate(id, symbol, interval, candle)
                    }),
                );
            } else {
                subs.push(
                    Subscription::run_with((id, symbol, interval), ws_candle_stream_keyed).map(
                        |(id, symbol, interval, candle)| {
                            Message::ChartWsCandleUpdate(id, symbol, interval, candle)
                        },
                    ),
                );
            }
        }
        for (symbol, id) in asset_ctx_streams {
            if let Some(api_key) = hydromancer_key.clone() {
                subs.push(
                    Subscription::run_with(
                        (api_key, id, symbol),
                        ws_hydromancer_asset_ctx_stream_keyed,
                    )
                    .map(|(id, symbol, ctx)| Message::ChartWsAssetCtxUpdate(id, symbol, ctx)),
                );
            } else {
                subs.push(
                    Subscription::run_with((id, symbol), ws_asset_ctx_stream_keyed)
                        .map(|(id, symbol, ctx)| Message::ChartWsAssetCtxUpdate(id, symbol, ctx)),
                );
            }
        }
    }
}
