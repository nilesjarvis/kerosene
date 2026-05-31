use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(in crate::spaghetti_update) fn reload_spaghetti_chart(
        &mut self,
        id: SpaghettiChartId,
    ) -> Task<Message> {
        let mut tasks = Vec::new();
        let chart_backfill_source = self.chart_backfill_source;
        let hydromancer_api_key = self.hydromancer_api_key.trim().to_string();
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            Self::normalize_spaghetti_session_granularity(inst, Self::now_ms());
            let target_tf = Self::spaghetti_effective_timeframe_for(
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                Self::now_ms(),
            );

            for series in &mut inst.canvas.series {
                let key = (series.symbol.clone(), target_tf);
                self.candle_data_cache.remove(&key);
                self.candle_data_cache_order.retain(|k| k != &key);

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
                        hydromancer_api_key.clone(),
                    ),
                ));
            }
            inst.canvas.cache.clear();
        }
        Task::batch(tasks)
    }
}
