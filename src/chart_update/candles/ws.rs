use crate::api::{Candle, is_valid_candle, open_time_starts_after_gap};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::chart_update::is_spot_asset_context_symbol;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

const SPOT_CANDLE_GAP_RELOAD_BACKOFF_MS: u64 = 60_000;

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
        let symbol_is_spot = self.is_spot_coin(&symbol) || is_spot_asset_context_symbol(&symbol);
        let symbol_allows_sparse_intervals = symbol_is_spot
            || self.is_outcome_coin(&symbol)
            || crate::schwab::is_schwab_symbol_key(&symbol)
            || interval == Timeframe::Mo1.api_str();
        let mut refresh_funding_ids = Vec::new();
        let mut primary_rollover = false;
        let mut secondary_updated = false;
        let mut primary_reload_ids = Vec::new();
        let mut secondary_reload_ids = Vec::new();

        for (chart_id, instance) in &mut self.charts {
            let interval_matches = instance.interval.api_str() == interval;
            let interval_ms = instance.interval.duration_ms();
            if instance.symbol == symbol && interval_matches {
                let last_open_time = instance.chart.candles.last().map(|last| last.open_time);
                let out_of_order = last_open_time.is_some_and(|last| candle.open_time < last);
                let has_gap = last_open_time.is_some_and(|last| {
                    open_time_starts_after_gap(last, candle.open_time, interval_ms)
                });
                let has_exact_interval_discontinuity = !symbol_allows_sparse_intervals
                    && last_open_time.is_some_and(|last| {
                        candle.open_time > last
                            && candle.open_time != last.saturating_add(interval_ms)
                    });
                let spot_gap_reload_due =
                    instance
                        .spot_candle_gap_reloaded_at_ms
                        .is_none_or(|last_reload_ms| {
                            now_ms.saturating_sub(last_reload_ms)
                                >= SPOT_CANDLE_GAP_RELOAD_BACKOFF_MS
                        });
                if out_of_order
                    || has_exact_interval_discontinuity
                    || (has_gap && (!symbol_allows_sparse_intervals || spot_gap_reload_due))
                {
                    // A live candle that jumps past the tail (reconnect after a
                    // sleep/quiet outage) may mean missed candles. Spot markets
                    // can also be naturally sparse, so reconcile once and then
                    // back off instead of reloading on every sparse update.
                    if symbol_allows_sparse_intervals {
                        instance.spot_candle_gap_reloaded_at_ms = Some(now_ms);
                    }
                    primary_reload_ids.push(*chart_id);
                } else {
                    let previous_close = instance.chart.candles.last().map(|candle| candle.close);
                    let next_close = candle.close;
                    let push_result = instance.chart.push_candle(candle.clone());
                    if push_result.applied() {
                        instance.chart.status = ChartStatus::Loaded;
                        instance.remember_primary_ws_candle(candle.clone(), now_ms);
                        instance.candle_interval_gap |= symbol_allows_sparse_intervals && has_gap;
                        primary_rollover |= push_result.appended();
                        if symbol_is_spot
                            && now_ms.saturating_sub(candle.close_time)
                                <= instance.interval.cache_display_max_age_ms()
                            && instance.candle_fetch_error.as_deref().is_some_and(|error| {
                                error.starts_with("Latest spot trade candle is stale")
                            })
                        {
                            instance.candle_fetch_error = None;
                        }
                        if should_flash {
                            instance.track_last_price_update(previous_close, next_close, now_ms);
                        }
                        if instance.macro_indicators.show_funding_rate {
                            refresh_funding_ids.push(*chart_id);
                        }
                    }
                }
            }
            if instance.secondary_symbol.as_deref() == Some(symbol.as_str()) && interval_matches {
                let secondary_last_open = instance
                    .chart
                    .secondary_series
                    .as_ref()
                    .and_then(|series| series.candles.last())
                    .map(|candle| candle.open_time);
                let out_of_order = secondary_last_open.is_some_and(|last| candle.open_time < last);
                let has_gap = secondary_last_open.is_some_and(|last| {
                    open_time_starts_after_gap(last, candle.open_time, interval_ms)
                });
                let has_exact_interval_discontinuity = !symbol_allows_sparse_intervals
                    && secondary_last_open.is_some_and(|last| {
                        candle.open_time > last
                            && candle.open_time != last.saturating_add(interval_ms)
                    });
                let spot_gap_reload_due = instance
                    .secondary_spot_candle_gap_reloaded_at_ms
                    .is_none_or(|last_reload_ms| {
                        now_ms.saturating_sub(last_reload_ms) >= SPOT_CANDLE_GAP_RELOAD_BACKOFF_MS
                    });
                if out_of_order
                    || has_exact_interval_discontinuity
                    || (has_gap && (!symbol_allows_sparse_intervals || spot_gap_reload_due))
                {
                    if symbol_allows_sparse_intervals {
                        instance.secondary_spot_candle_gap_reloaded_at_ms = Some(now_ms);
                    }
                    secondary_reload_ids.push(*chart_id);
                } else {
                    let push_result = instance.chart.push_secondary_candle(candle.clone());
                    if push_result.applied() {
                        instance.remember_secondary_ws_candle(candle.clone(), now_ms);
                        instance.secondary_candle_interval_gap |=
                            symbol_allows_sparse_intervals && has_gap;
                        secondary_updated |= push_result.appended();
                    }
                }
            }
        }

        if primary_rollover {
            self.cache_primary_candles_for(&symbol, &interval);
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
                (instance.symbol == symbol && instance.interval.api_str() == interval)
                    .then_some(*chart_id)
            })
            .collect::<Vec<_>>();
        let secondary_reload_ids = self
            .charts
            .iter()
            .filter_map(|(chart_id, instance)| {
                (instance.secondary_symbol.as_deref() == Some(symbol.as_str())
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

    fn cache_primary_candles_for(&mut self, symbol: &str, interval: &str) {
        // Duplicate chart panes can be at different stages of cold-start
        // hydration. Prefer the most recently provider-verified instance so an
        // unverified pane cannot nondeterministically replace the shared cache.
        let cache = self
            .charts
            .values()
            .filter(|instance| {
                instance.symbol == symbol
                    && instance.interval.api_str() == interval
                    && !instance.chart.candles.is_empty()
            })
            .max_by_key(|instance| {
                (
                    instance.candle_history_verified_at_ms,
                    instance.candle_ws_updated_at_ms,
                    instance.chart.candles.len(),
                )
            })
            .map(|instance| (instance.interval, instance.chart.candles.clone()));
        if let Some((timeframe, candles)) = cache {
            self.cache_candles(symbol, timeframe, candles);
        }
    }

    fn cache_secondary_candles_for(&mut self, symbol: &str, interval: &str) {
        let cache = self
            .charts
            .values()
            .filter(|instance| {
                instance.secondary_symbol.as_deref() == Some(symbol)
                    && instance.interval.api_str() == interval
                    && instance
                        .chart
                        .secondary_series
                        .as_ref()
                        .is_some_and(|series| !series.candles.is_empty())
            })
            .max_by_key(|instance| {
                (
                    instance.secondary_candle_history_verified_at_ms,
                    instance.secondary_candle_ws_updated_at_ms,
                    instance
                        .chart
                        .secondary_series
                        .as_ref()
                        .map_or(0, |series| series.candles.len()),
                )
            })
            .and_then(|instance| {
                instance
                    .chart
                    .secondary_series
                    .as_ref()
                    .map(|series| (instance.interval, series.candles.clone()))
            });
        if let Some((timeframe, candles)) = cache {
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
