use crate::api::{Candle, is_valid_candle};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn apply_chart_ws_candle_update(
        &mut self,
        _id: ChartId,
        symbol: String,
        interval: String,
        source_context: crate::read_data_provider::MarketDataSourceContext,
        candle: Candle,
    ) -> Task<Message> {
        if !self.chart_candle_stream_source_is_current(&interval, source_context) {
            return Task::none();
        }
        if self.symbol_key_is_hidden(&symbol) {
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let should_flash = is_valid_candle(&candle);
        let mut refresh_funding_ids = Vec::new();
        let mut secondary_updated = false;

        for (chart_id, instance) in &mut self.charts {
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && instance.symbol == symbol
                && instance.interval.api_str() == interval
            {
                let previous_close = instance.chart.candles.last().map(|candle| candle.close);
                let next_close = candle.close;
                instance.chart.push_candle(candle.clone());
                if should_flash {
                    instance.track_last_price_update(previous_close, next_close, now_ms);
                }
                if instance.macro_indicators.show_funding_rate {
                    refresh_funding_ids.push(*chart_id);
                }
            }
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && instance.secondary_symbol.as_deref() == Some(symbol.as_str())
                && instance.interval.api_str() == interval
            {
                instance.chart.push_secondary_candle(candle.clone());
                secondary_updated = true;
            }
        }

        if secondary_updated {
            self.cache_secondary_candles_for(&symbol, &interval);
        }

        if !refresh_funding_ids.is_empty() {
            return Task::batch(
                refresh_funding_ids
                    .into_iter()
                    .map(|chart_id| self.maybe_fetch_chart_funding(chart_id)),
            );
        }
        Task::none()
    }

    pub(in crate::chart_update) fn apply_chart_ws_candle_lagged(
        &mut self,
        _id: ChartId,
        symbol: String,
        interval: String,
        source_context: crate::read_data_provider::MarketDataSourceContext,
        _skipped: u64,
    ) -> Task<Message> {
        if !self.chart_candle_stream_source_is_current(&interval, source_context) {
            return Task::none();
        }
        if self.symbol_key_is_hidden(&symbol) {
            return Task::none();
        }

        let reload_ids = self
            .charts
            .iter()
            .filter_map(|(chart_id, instance)| {
                (matches!(instance.chart.status, ChartStatus::Loaded)
                    && instance.symbol == symbol
                    && instance.interval.api_str() == interval)
                    .then_some(*chart_id)
            })
            .collect::<Vec<_>>();
        let secondary_reload_ids = self
            .charts
            .iter()
            .filter_map(|(chart_id, instance)| {
                (matches!(instance.chart.status, ChartStatus::Loaded)
                    && instance.secondary_symbol.as_deref() == Some(symbol.as_str())
                    && instance.interval.api_str() == interval)
                    .then_some(*chart_id)
            })
            .collect::<Vec<_>>();

        if reload_ids.is_empty() && secondary_reload_ids.is_empty() {
            return Task::none();
        }

        let mut tasks = Vec::with_capacity(reload_ids.len() + secondary_reload_ids.len());
        for chart_id in reload_ids {
            tasks.push(self.reload_chart_candles(chart_id));
        }
        for chart_id in secondary_reload_ids {
            tasks.push(self.reload_chart_secondary_candles(chart_id));
        }
        Task::batch(tasks)
    }

    fn cache_secondary_candles_for(&mut self, symbol: &str, interval: &str) {
        let mut caches = Vec::new();
        for instance in self.charts.values() {
            if instance.secondary_symbol.as_deref() == Some(symbol)
                && instance.interval.api_str() == interval
                && let Some(series) = instance.chart.secondary_series.as_ref()
                && !series.candles.is_empty()
            {
                caches.push((instance.interval, series.candles.clone()));
            }
        }
        for (timeframe, candles) in caches {
            self.cache_candles(symbol, timeframe, candles);
        }
    }

    fn chart_candle_stream_source_is_current(
        &self,
        interval: &str,
        source_context: crate::read_data_provider::MarketDataSourceContext,
    ) -> bool {
        if interval == Timeframe::S1.api_str() {
            self.hydromancer_keyed_market_stream_source_is_current(source_context)
        } else {
            self.market_stream_source_is_current(source_context)
        }
    }
}

#[cfg(test)]
mod tests;
