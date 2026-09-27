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
        self.repair_spaghetti_series(context.chart_id, &context.symbol, false)
    }

    pub(crate) fn repair_spaghetti_series(
        &mut self,
        id: SpaghettiChartId,
        symbol: &str,
        force: bool,
    ) -> Task<Message> {
        if self.symbol_key_is_hidden(symbol) {
            return Task::none();
        }
        let backfill = ChartBackfillFetchContext::new(
            self.chart_backfill_source,
            self.read_data_provider_generation,
            self.hydromancer_key_generation,
            self.hydromancer_api_key_for_task(),
        );
        let Some(inst) = self.spaghetti_charts.get_mut(&id) else {
            return Task::none();
        };
        let health = inst.health.entry(symbol.to_string()).or_default();
        if health.pending.is_some() || (!force && health.next_retry_ms > Self::now_ms()) {
            return Task::none();
        }
        health.next_retry_ms = Self::now_ms().saturating_add(60_000);
        Self::fetch_spaghetti_candles(
            id,
            self.spaghetti_instance_epoch,
            symbol,
            inst.interval,
            inst.canvas.active_session,
            inst.session_granularity,
            backfill,
        )
    }

    pub(in crate::spaghetti_update) fn reload_spaghetti_chart(
        &mut self,
        id: SpaghettiChartId,
    ) -> Task<Message> {
        let symbols = self
            .spaghetti_charts
            .get(&id)
            .map(|inst| {
                inst.canvas
                    .series
                    .iter()
                    .map(|series| series.symbol.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Task::batch(
            symbols
                .into_iter()
                .map(|symbol| self.repair_spaghetti_series(id, &symbol, true))
                .collect::<Vec<_>>(),
        )
    }
}
