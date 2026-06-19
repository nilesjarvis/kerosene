use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn reload_chart_candles(&mut self, id: ChartId) -> Task<Message> {
        let symbol = self
            .charts
            .get(&id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        let tf = self
            .charts
            .get(&id)
            .map(|inst| inst.interval)
            .unwrap_or(Timeframe::H1);

        if symbol.is_empty() {
            return Task::none();
        }

        self.remove_cached_candles(&symbol, tf);
        self.clear_chart_heatmap_pending_request_state(id);
        self.clear_chart_liquidation_pending_request_state(id);

        if let Some(instance) = self.charts.get_mut(&id) {
            // Clear before refetching so the reload REPLACES the series rather
            // than merging a fresh window onto a stale block — merging would only
            // shrink an interior gap, not heal it. The fresh fetch repopulates
            // from scratch and older history is restored by backfill.
            instance.chart.candles.clear();
            instance.chart.status = ChartStatus::Loading;
            instance.candle_fetch_error = None;
            instance.candle_backfill_exhausted = false;
            Self::clear_chart_market_display_state(instance);
            instance.chart.candle_cache.clear();
        }

        let mut tasks = vec![self.queue_candle_fetch_for(id, &symbol, tf, None)];
        let secondary_task = self.reload_chart_secondary_candles(id);
        tasks.push(secondary_task);
        Task::batch(tasks)
    }

    pub(in crate::chart_update) fn reload_chart_secondary_candles(
        &mut self,
        id: ChartId,
    ) -> Task<Message> {
        let Some((symbol, tf)) = self.charts.get(&id).and_then(|inst| {
            inst.secondary_symbol
                .clone()
                .map(|symbol| (symbol, inst.interval))
        }) else {
            return Task::none();
        };

        if self.symbol_key_is_hidden(&symbol) {
            return Task::none();
        }

        self.remove_cached_candles(&symbol, tf);

        if let Some(instance) = self.charts.get_mut(&id) {
            instance.secondary_candle_fetch_error = None;
            instance.secondary_candle_backfill_exhausted = false;
            if instance.chart.secondary_series.is_none()
                && let Some(symbol) = instance.secondary_symbol.clone()
            {
                let display = instance
                    .secondary_symbol_display
                    .clone()
                    .unwrap_or_else(|| symbol.split(':').nth(1).unwrap_or(&symbol).to_string());
                instance
                    .chart
                    .set_secondary_series_identity(symbol, display);
            }
            instance.chart.set_secondary_candles(Vec::new());
        }

        self.queue_secondary_candle_fetch_for(id, &symbol, tf, None)
    }
}
