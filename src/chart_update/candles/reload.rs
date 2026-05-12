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

        let key = (symbol.clone(), tf);
        self.candle_data_cache.remove(&key);
        self.candle_data_cache_order.retain(|k| k != &key);

        if let Some(instance) = self.charts.get_mut(&id) {
            if instance.chart.candles.is_empty() {
                instance.chart.status = ChartStatus::Loading;
            } else {
                instance.chart.status = ChartStatus::Loaded;
            }
            instance.candle_fetch_error = None;
            instance.heatmap_last_fetch = None;
            instance.heatmap_viewport = None;
            instance.heatmap_status = None;
            instance.heatmap_fetching = false;
            instance.last_price_flash = None;
            Self::clear_heatmap_display(instance);
            Self::clear_liquidation_display(instance);
            Self::clear_funding_display(instance);
            instance.chart.candle_cache.clear();
        }

        self.queue_candle_fetch_for(id, &symbol, tf, None)
    }
}
