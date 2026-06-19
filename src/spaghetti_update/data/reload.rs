use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiWsCandleContext};

use iced::Task;

impl TradingTerminal {
    pub(in crate::spaghetti_update) fn reload_spaghetti_chart_after_ws_lag(
        &mut self,
        context: SpaghettiWsCandleContext,
    ) -> Task<Message> {
        if !self.spaghetti_ws_candle_context_is_current(&context) {
            return Task::none();
        }

        let should_reload = self
            .spaghetti_charts
            .get(&context.chart_id)
            .is_some_and(|inst| {
                inst.canvas
                    .series
                    .iter()
                    .any(|series| series.loaded && series.symbol == context.symbol)
            });

        if !should_reload {
            return Task::none();
        }

        self.reload_spaghetti_chart(context.chart_id)
    }

    pub(in crate::spaghetti_update) fn reload_spaghetti_chart(
        &mut self,
        id: SpaghettiChartId,
    ) -> Task<Message> {
        let mut tasks = Vec::new();
        let chart_backfill_source = self.chart_backfill_source;
        let read_data_provider_generation = self.read_data_provider_generation;
        let hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        let mut removed_cache_keys = Vec::new();
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            Self::normalize_spaghetti_session_granularity(inst, Self::now_ms());
            let target_tf = Self::spaghetti_effective_timeframe_for(
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                Self::now_ms(),
            );

            for series in &mut inst.canvas.series {
                removed_cache_keys.push((series.symbol.clone(), target_tf));

                series.candles.clear();
                series.loaded = false;

                tasks.push(Self::fetch_spaghetti_candles(
                    id,
                    &series.symbol,
                    inst.interval,
                    inst.canvas.active_session,
                    inst.session_granularity,
                    None,
                    ChartBackfillFetchContext::new(
                        chart_backfill_source,
                        read_data_provider_generation,
                        hydromancer_generation,
                        hydromancer_api_key.clone(),
                    ),
                ));
            }
            inst.canvas.cache.clear();
        }
        for (symbol, timeframe) in removed_cache_keys {
            self.remove_cached_candles(&symbol, timeframe);
        }
        Task::batch(tasks)
    }
}
