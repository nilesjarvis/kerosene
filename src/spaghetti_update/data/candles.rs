use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiCandleFetch, SpaghettiChartId};

use iced::Task;

impl TradingTerminal {
    pub(in crate::spaghetti_update) fn apply_spaghetti_candles_loaded(
        &mut self,
        request: SpaghettiCandleFetch,
        result: Result<Vec<Candle>, String>,
    ) -> Task<Message> {
        let symbol = request.symbol;
        if self.symbol_key_is_hidden(&symbol) {
            return Task::none();
        }
        let mut new_cache_data = None;
        let mut remove_cache_data = None;

        if let Some(inst) = self.spaghetti_charts.get_mut(&request.chart_id)
            && let Some(series) = inst.canvas.series.iter_mut().find(|s| s.symbol == symbol)
        {
            let current_tf = Self::spaghetti_effective_timeframe_for(
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                Self::now_ms(),
            );
            if current_tf != request.timeframe
                || inst.canvas.active_session != request.session
                || inst.session_granularity != request.session_granularity
            {
                return Task::none();
            }

            match result {
                Ok(mut new_candles) => {
                    if series.candles.is_empty() {
                        series.candles = new_candles;
                    } else if let Some(first_new) = new_candles.first().map(|c| c.open_time) {
                        series.candles.retain(|c| c.open_time < first_new);
                        series.candles.append(&mut new_candles);
                    }
                    if series.candles.len() > 10000 {
                        let trim = series.candles.len() - 10000;
                        series.candles.drain(0..trim);
                    }
                    series.loaded = true;

                    new_cache_data = Some((
                        series.symbol.clone(),
                        request.timeframe,
                        series.candles.clone(),
                    ));
                }
                Err(_) => {
                    series.loaded = false;
                    remove_cache_data = Some((series.symbol.clone(), request.timeframe));
                }
            }
            if inst.pair_mode {
                inst.canvas.reset_epoch = inst.canvas.reset_epoch.saturating_add(1);
            }
            Self::refresh_spaghetti_session_anchor(inst);
            inst.canvas.cache.clear();
        }

        if let Some((sym, tf, new_cache)) = new_cache_data {
            self.cache_candles(&sym, tf, new_cache);
        } else if let Some((sym, tf)) = remove_cache_data {
            let key = (sym, tf);
            self.candle_data_cache.remove(&key);
            self.candle_data_cache_order.retain(|k| k != &key);
        }

        Task::none()
    }

    pub(in crate::spaghetti_update) fn apply_spaghetti_ws_candle_update(
        &mut self,
        id: SpaghettiChartId,
        symbol: String,
        candle: Candle,
    ) -> Task<Message> {
        if self.symbol_key_is_hidden(&symbol) {
            return Task::none();
        }
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.canvas.push_candle(&symbol, candle);
            Self::refresh_spaghetti_session_anchor(inst);
        }
        Task::none()
    }
}
