use crate::api::{Candle, is_valid_candle, open_time_starts_after_gap};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn apply_orderbook_tick_price_to_charts(
        &mut self,
        symbol: &str,
        price: f64,
        now_ms: u64,
    ) {
        if !price.is_finite() || price <= 0.0 {
            return;
        }

        let mut secondary_updated = false;
        for instance in self.charts.values_mut() {
            if !instance.interval.uses_orderbook_tick_candles() {
                continue;
            }

            if instance.symbol == symbol {
                let candle = orderbook_tick_candle(&instance.chart.candles, price, now_ms);
                let previous_close = instance.chart.candles.last().map(|candle| candle.close);
                instance.chart.push_candle(candle);
                instance.chart.status = ChartStatus::Loaded;
                instance.track_last_price_update(previous_close, price, now_ms);
            }

            if instance.secondary_symbol.as_deref() == Some(symbol) {
                let prior = instance
                    .chart
                    .secondary_series
                    .as_ref()
                    .map(|series| series.candles.as_slice())
                    .unwrap_or(&[]);
                let candle = orderbook_tick_candle(prior, price, now_ms);
                instance.chart.push_secondary_candle(candle);
                secondary_updated = true;
            }
        }

        if secondary_updated {
            self.cache_secondary_candles_for(symbol, Timeframe::Tick.api_str());
        }
    }

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
        let mut primary_reload_ids = Vec::new();
        let mut secondary_reload_ids = Vec::new();

        for (chart_id, instance) in &mut self.charts {
            let interval_matches = instance.interval.api_str() == interval;
            let interval_ms = instance.interval.duration_ms();
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && instance.symbol == symbol
                && interval_matches
            {
                if instance.chart.candles.last().is_some_and(|last| {
                    open_time_starts_after_gap(last.open_time, candle.open_time, interval_ms)
                }) {
                    // A live candle that jumps past the tail (reconnect after a
                    // sleep/quiet outage) means missed candles. Blind-appending
                    // would splice a phantom gap that then persists; reload to
                    // refetch a contiguous window instead.
                    primary_reload_ids.push(*chart_id);
                } else {
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
            }
            if matches!(instance.chart.status, ChartStatus::Loaded)
                && instance.secondary_symbol.as_deref() == Some(symbol.as_str())
                && interval_matches
            {
                let secondary_last_open = instance
                    .chart
                    .secondary_series
                    .as_ref()
                    .and_then(|series| series.candles.last())
                    .map(|candle| candle.open_time);
                if secondary_last_open.is_some_and(|last| {
                    open_time_starts_after_gap(last, candle.open_time, interval_ms)
                }) {
                    secondary_reload_ids.push(*chart_id);
                } else {
                    instance.chart.push_secondary_candle(candle.clone());
                    secondary_updated = true;
                }
            }
        }

        if secondary_updated {
            self.cache_secondary_candles_for(&symbol, &interval);
        }

        let mut tasks = Vec::new();
        for chart_id in primary_reload_ids {
            tasks.push(self.reload_chart_candles(chart_id));
        }
        for chart_id in secondary_reload_ids {
            tasks.push(self.reload_chart_secondary_candles(chart_id));
        }
        for chart_id in refresh_funding_ids {
            tasks.push(self.maybe_fetch_chart_funding(chart_id));
        }
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
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

fn orderbook_tick_candle(prior: &[Candle], price: f64, now_ms: u64) -> Candle {
    let open_time = prior
        .last()
        .map(|candle| {
            candle
                .open_time
                .saturating_add(Timeframe::Tick.duration_ms())
        })
        .unwrap_or(1)
        .max(now_ms.max(1));
    Candle {
        open_time,
        close_time: open_time,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 0.0,
    }
}

#[cfg(test)]
mod tests;
