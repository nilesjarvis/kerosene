use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(in crate::spaghetti_update) fn reload_spaghetti_chart(
        &mut self,
        id: SpaghettiChartId,
    ) -> Task<Message> {
        let mut tasks = Vec::new();
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
                ));
            }
            inst.canvas.cache.clear();
        }
        Task::batch(tasks)
    }
}
