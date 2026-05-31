use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(in crate::spaghetti_update) fn switch_spaghetti_timeframe(
        &mut self,
        id: SpaghettiChartId,
        tf: Timeframe,
    ) -> Task<Message> {
        let mut cache_tasks = Vec::new();
        if let Some(inst) = self.spaghetti_charts.get(&id) {
            let old_tf = Self::spaghetti_effective_timeframe_for(
                inst.interval,
                inst.canvas.active_session,
                inst.session_granularity,
                Self::now_ms(),
            );
            for series in &inst.canvas.series {
                cache_tasks.push((old_tf, series.symbol.clone(), series.candles.clone()));
            }
        }
        for (old_tf, symbol, candles) in cache_tasks {
            self.cache_candles(&symbol, old_tf, candles);
        }

        let mut to_load = Vec::new();
        let mut inst_interval = Timeframe::H1;
        let mut inst_active_session = None;
        let mut inst_session_granularity = None;

        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            let in_anchor_mode = inst.canvas.active_session.is_some();
            let has_unloaded_series = inst.canvas.series.iter().any(|series| !series.loaded);

            if in_anchor_mode {
                let requested_granularity = if let Some(session) = inst.canvas.active_session {
                    let start = session.last_open_ms(Self::now_ms());
                    let span = Self::now_ms().saturating_sub(start);
                    Self::spaghetti_session_granularity_fits(span, tf).then_some(tf)
                } else {
                    Some(tf)
                };

                if inst.session_granularity == requested_granularity && !has_unloaded_series {
                    return Task::none();
                }
                inst.session_granularity = requested_granularity;
            } else if tf != inst.interval || has_unloaded_series {
                inst.interval = tf;
            } else {
                return Task::none();
            }

            inst_interval = inst.interval;
            inst_active_session = inst.canvas.active_session;
            inst_session_granularity = inst.session_granularity;
            for series in &inst.canvas.series {
                to_load.push(series.symbol.clone());
            }
        }

        let mut tasks = Vec::new();
        let mut cached_updates = Vec::new();
        let chart_backfill_source = self.chart_backfill_source;
        let hydromancer_api_key = self.hydromancer_api_key.trim().to_string();
        let target_tf = Self::spaghetti_effective_timeframe_for(
            inst_interval,
            inst_active_session,
            inst_session_granularity,
            Self::now_ms(),
        );

        for symbol in to_load {
            let mut cached_last_time = None;
            if let Some(cached_candles) = self.get_cached_candles(&symbol, target_tf) {
                cached_last_time = cached_candles.last().map(|c| c.open_time);
                cached_updates.push((symbol.clone(), cached_candles));
            }
            tasks.push(Self::fetch_spaghetti_candles(
                id,
                &symbol,
                inst_interval,
                inst_active_session,
                inst_session_granularity,
                cached_last_time,
                ChartBackfillFetchContext::new(chart_backfill_source, hydromancer_api_key.clone()),
            ));
        }

        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            for series in &mut inst.canvas.series {
                if let Some((_, candles)) = cached_updates.iter().find(|(s, _)| s == &series.symbol)
                {
                    series.candles = candles.clone();
                    series.loaded = true;
                } else {
                    series.candles.clear();
                    series.loaded = false;
                }
            }
            Self::refresh_spaghetti_session_anchor(inst);
            if inst.pair_mode {
                inst.canvas.reset_epoch = inst.canvas.reset_epoch.saturating_add(1);
            }
            inst.canvas.cache.clear();
        }

        self.persist_config();
        Task::batch(tasks)
    }
}
