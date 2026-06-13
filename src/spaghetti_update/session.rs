use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(super) fn set_spaghetti_session(
        &mut self,
        id: SpaghettiChartId,
        session: Option<spaghetti::Session>,
    ) -> Task<Message> {
        let chart_backfill_source = self.chart_backfill_source;
        let read_data_provider_generation = self.read_data_provider_generation;
        let hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.canvas.active_session = session;
            Self::normalize_spaghetti_session_granularity(inst, Self::now_ms());
            Self::refresh_spaghetti_session_anchor(inst);
            inst.canvas.reset_epoch = inst.canvas.reset_epoch.saturating_add(1);
            inst.canvas.cache.clear();

            let mut tasks = Vec::new();
            for series in &mut inst.canvas.series {
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
            self.persist_config();
            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
        }
        Task::none()
    }

    pub(super) fn set_spaghetti_session_granularity_auto(
        &mut self,
        id: SpaghettiChartId,
    ) -> Task<Message> {
        let chart_backfill_source = self.chart_backfill_source;
        let read_data_provider_generation = self.read_data_provider_generation;
        let hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            if inst.session_granularity.is_none() {
                return Task::none();
            }
            inst.session_granularity = None;

            if inst.canvas.active_session.is_none() {
                self.persist_config();
                return Task::none();
            }

            let mut tasks = Vec::new();
            for series in &mut inst.canvas.series {
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
            Self::refresh_spaghetti_session_anchor(inst);
            inst.canvas.reset_epoch = inst.canvas.reset_epoch.saturating_add(1);
            inst.canvas.cache.clear();
            self.persist_config();
            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
        }
        Task::none()
    }

    pub(super) fn reset_spaghetti_view(&mut self, id: SpaghettiChartId) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.canvas.reset_epoch = inst.canvas.reset_epoch.saturating_add(1);
            inst.canvas.cache.clear();
        }
        Task::none()
    }
}
