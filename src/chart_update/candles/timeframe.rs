use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn switch_chart_candle_timeframe(
        &mut self,
        id: ChartId,
        tf: Timeframe,
    ) -> Task<Message> {
        let should_switch = self.charts.get(&id).is_some_and(|inst| {
            tf != inst.interval
                || matches!(inst.chart.status, ChartStatus::Error(_))
                || inst.chart.candles.is_empty()
        });
        if !should_switch {
            return Task::none();
        }

        let mut old_cache_data = None;
        if let Some(instance) = self.charts.get(&id) {
            // Only save to cache if we actually have a fully loaded chart.
            if !instance.chart.candles.is_empty()
                && matches!(instance.chart.status, ChartStatus::Loaded)
            {
                old_cache_data = Some((
                    instance.interval,
                    instance.symbol.clone(),
                    instance.chart.candles.clone(),
                ));
            }
        }
        if let Some((old_tf, old_symbol, old_candles)) = old_cache_data {
            self.cache_candles(&old_symbol, old_tf, old_candles);
        }

        let symbol = self
            .charts
            .get(&id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        let mut cached_last_time = None;
        let cached_candles = self.get_cached_candles(&symbol, tf);

        if let Some(instance) = self.charts.get_mut(&id) {
            instance.interval = tf;
            instance.chart.set_timeframe(tf);
            instance.chart.clear_hud_armed();
            instance.heatmap_last_fetch = None;
            instance.heatmap_viewport = None;
            instance.heatmap_status = None;
            instance.heatmap_fetching = false;
            instance.last_price_flash = None;
            Self::clear_heatmap_display(instance);
            Self::clear_liquidation_display(instance);
            Self::clear_funding_display(instance);
            instance.chart.request_view_reset();
            if let Some(candles) = cached_candles {
                cached_last_time = candles.last().map(|c| c.open_time);
                instance.chart.set_candles(candles);
            } else {
                instance.chart.status = ChartStatus::Loading;
                instance.chart.candles.clear();
                instance.chart.candle_cache.clear();
            }
        }

        self.persist_config();
        if symbol.is_empty() {
            return Task::none();
        }
        self.queue_candle_fetch_for(id, &symbol, tf, cached_last_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::config::ChartCrosshairStyle;

    #[test]
    fn timeframe_switch_disarms_hud_trading() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        instance.chart.set_hud_armed_at(true, 1_000);
        terminal.charts.insert(1, instance);

        let _task = terminal.switch_chart_candle_timeframe(1, Timeframe::M15);

        assert!(!terminal.charts.get(&1).unwrap().chart.hud_armed());
    }
}
