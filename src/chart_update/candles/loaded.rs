use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::{CANDLE_FETCH_MAX_ATTEMPTS, CandleFetchRequest};
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn apply_chart_candles_loaded(
        &mut self,
        request: CandleFetchRequest,
        result: Result<Vec<Candle>, String>,
    ) -> Task<Message> {
        if self.is_ticker_muted(&request.symbol) {
            return Task::none();
        }
        let id = request.chart_id;
        let mut new_cache_data = None;
        let mut remove_cache_data = None;
        let mut retry_request = None;
        let mut fetch_overlays = false;

        if let Some(instance) = self.charts.get_mut(&id) {
            let request_matches = instance.symbol == request.symbol
                && instance.interval == request.timeframe
                && instance.candle_fetch_request.as_ref() == Some(&request);
            if !request_matches {
                return Task::none();
            }

            match result {
                Ok(candles) => {
                    instance.candle_fetch_request = None;
                    if candles.is_empty() {
                        if instance.chart.candles.is_empty() {
                            instance.chart.set_error(format!(
                                "No candle data returned for {} {}",
                                request.symbol, request.timeframe
                            ));
                            remove_cache_data = Some((request.symbol.clone(), request.timeframe));
                        } else {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error =
                                Some("No fresh candle data returned".to_string());
                        }
                    } else {
                        instance.candle_fetch_error = None;
                        instance.chart.merge_candles(candles);
                        fetch_overlays = true;
                        new_cache_data = Some((
                            request.symbol.clone(),
                            request.timeframe,
                            instance.chart.candles.clone(),
                        ));
                    }
                }
                Err(error) => {
                    let next_attempt = request.attempt.saturating_add(1);
                    if next_attempt < CANDLE_FETCH_MAX_ATTEMPTS {
                        let mut next_request = request.clone();
                        next_request.attempt = next_attempt;
                        next_request.end_ms = Self::now_ms();
                        instance.candle_fetch_request = Some(next_request.clone());
                        if instance.chart.candles.is_empty() {
                            instance.chart.status = ChartStatus::Loading;
                        } else {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error = Some(format!(
                                "Retrying candle refresh ({}/{})",
                                next_attempt + 1,
                                CANDLE_FETCH_MAX_ATTEMPTS
                            ));
                        }
                        retry_request = Some(next_request);
                    } else {
                        instance.candle_fetch_request = None;
                        if instance.chart.candles.is_empty() {
                            instance.chart.set_error(error);
                            remove_cache_data = Some((request.symbol.clone(), request.timeframe));
                        } else {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error = Some(error);
                        }
                    }
                }
            }
        }

        if let Some(request) = retry_request {
            return Self::fetch_candles_task(request);
        }

        if let Some((symbol, tf, new_cache)) = new_cache_data {
            self.sync_chart_position_for(id);
            self.sync_chart_orders_for(id);
            self.sync_chart_trade_markers_for(id);
            self.cache_candles(&symbol, tf, new_cache);
        } else if let Some((symbol, tf)) = remove_cache_data {
            let key = (symbol, tf);
            self.candle_data_cache.remove(&key);
            self.candle_data_cache_order.retain(|k| k != &key);
        }

        if fetch_overlays {
            let liq_task = self.maybe_fetch_liquidations(id);
            let heat_task = self.maybe_fetch_heatmap(id);
            let funding_task = self.maybe_fetch_chart_funding(id);
            return Task::batch([liq_task, heat_task, funding_task]);
        }

        Task::none()
    }
}
