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
        let mut old_secondary_cache_data = None;
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
            if let Some(series) = instance.chart.secondary_series.as_ref()
                && !series.candles.is_empty()
            {
                old_secondary_cache_data = Some((
                    instance.interval,
                    series.symbol_key.clone(),
                    series.candles.clone(),
                ));
            }
        }
        if let Some((old_tf, old_symbol, old_candles)) = old_cache_data {
            self.cache_candles(&old_symbol, old_tf, old_candles);
        }
        if let Some((old_tf, old_symbol, old_candles)) = old_secondary_cache_data {
            self.cache_candles(&old_symbol, old_tf, old_candles);
        }

        let symbol = self
            .charts
            .get(&id)
            .map(|inst| inst.symbol.clone())
            .unwrap_or_default();
        let secondary_symbol = self
            .charts
            .get(&id)
            .and_then(|inst| inst.secondary_symbol.clone());
        let mut cached_last_time = None;
        let cached_candles = self.get_cached_candles(&symbol, tf);
        let mut secondary_cached_last_time = None;
        let secondary_cached_candles = secondary_symbol
            .as_ref()
            .and_then(|symbol| self.get_cached_candles(symbol, tf));
        self.clear_chart_heatmap_pending_request_state(id);
        self.clear_chart_liquidation_pending_request_state(id);

        if let Some(instance) = self.charts.get_mut(&id) {
            instance.interval = tf;
            instance.chart.set_timeframe(tf);
            instance.chart.clear_hud_armed();
            Self::clear_chart_market_display_state(instance);
            instance.chart.request_view_reset();
            if let Some(candles) = cached_candles {
                cached_last_time = candles.last().map(|c| c.open_time);
                instance.chart.set_candles(candles);
            } else {
                instance.chart.status = ChartStatus::Loading;
                instance.chart.candles.clear();
                instance.chart.candle_cache.clear();
            }
            if let Some(candles) = secondary_cached_candles {
                secondary_cached_last_time = candles.last().map(|c| c.open_time);
                instance.chart.set_secondary_candles(candles);
            } else if instance.secondary_symbol.is_some() {
                instance.chart.set_secondary_candles(Vec::new());
            }
            instance.secondary_candle_fetch_request = None;
            instance.secondary_candle_fetch_error = None;
            instance.spot_candle_gap_reloaded_at_ms = None;
            instance.secondary_spot_candle_gap_reloaded_at_ms = None;
        }

        self.persist_config();
        let mut tasks = Vec::new();
        if symbol.is_empty() {
            return if let Some(symbol) = secondary_symbol {
                self.queue_secondary_candle_fetch_for(id, &symbol, tf, secondary_cached_last_time)
            } else {
                Task::none()
            };
        }
        tasks.push(self.queue_candle_fetch_for(id, &symbol, tf, cached_last_time));
        if let Some(symbol) = secondary_symbol {
            tasks.push(self.queue_secondary_candle_fetch_for(
                id,
                &symbol,
                tf,
                secondary_cached_last_time,
            ));
        }
        Task::batch(tasks)
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

    #[test]
    fn timeframe_switch_prunes_chart_from_hyperdash_pending_requests() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        terminal
            .charts
            .insert(2, ChartInstance::new(2, "ETH".to_string(), Timeframe::H1));
        terminal
            .heatmap_pending_charts
            .insert("heat-shared".to_string(), vec![1, 2]);
        terminal
            .heatmap_pending_charts
            .insert("heat-only".to_string(), vec![1]);
        terminal
            .liquidation_pending_charts
            .insert("liq-shared".to_string(), vec![1, 2]);
        terminal
            .liquidation_pending_charts
            .insert("liq-only".to_string(), vec![1]);

        let _task = terminal.switch_chart_candle_timeframe(1, Timeframe::M15);

        assert_eq!(
            terminal.heatmap_pending_charts.get("heat-shared"),
            Some(&vec![2])
        );
        assert!(!terminal.heatmap_pending_charts.contains_key("heat-only"));
        assert_eq!(
            terminal.liquidation_pending_charts.get("liq-shared"),
            Some(&vec![2])
        );
        assert!(!terminal.liquidation_pending_charts.contains_key("liq-only"));
    }
}
