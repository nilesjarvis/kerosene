use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::timeframe::Timeframe;

use iced::Task;

impl TradingTerminal {
    pub(super) fn switch_all_chart_timeframes(&mut self, tf: Timeframe) -> Task<Message> {
        // Instance maps include main panes, canvas workspaces, and detached charts.
        // Reuse the individual handlers so caches, stale requests, and persistence
        // follow the same path as a chart's own timeframe control.
        let chart_ids: Vec<_> = self.charts.keys().copied().collect();
        let spaghetti_ids: Vec<_> = self.spaghetti_charts.keys().copied().collect();
        let mut tasks = Vec::with_capacity(chart_ids.len() + spaghetti_ids.len());
        for id in chart_ids {
            tasks.push(self.switch_chart_candle_timeframe(id, tf));
        }
        for id in spaghetti_ids {
            tasks.push(self.update_spaghetti(Message::SpaghettiSwitchTimeframe(id, tf)));
        }
        Task::batch(tasks)
    }

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
            instance.reset_primary_candle_trust();
            instance.reset_secondary_candle_trust();
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
    use crate::api::Candle;
    use crate::chart_state::{ChartInstance, DetachedChartWindowState};
    use crate::config::{ChartCrosshairStyle, KeroseneConfig};
    use crate::pane_state::PaneKind;
    use crate::spaghetti_state::SpaghettiChartInstance;

    fn empty_terminal() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
        terminal.charts.clear();
        terminal.spaghetti_charts.clear();
        terminal.config_save_due_at = None;
        terminal
    }

    #[test]
    fn global_timeframe_updates_all_workspaces_and_keeps_individual_controls_independent() {
        let mut terminal = empty_terminal();
        for (id, symbol, timeframe) in [
            (1, "BTC", Timeframe::M5),
            (2, "ETH", Timeframe::H1),
            (3, "SOL", Timeframe::D1),
        ] {
            let mut instance = ChartInstance::new(id, symbol.into(), timeframe);
            instance.chart.set_crosshair_style(ChartCrosshairStyle::Hud);
            instance.chart.set_hud_armed_at(true, 1_000);
            terminal.charts.insert(id, instance);
        }
        terminal.insert_test_canvas_pane(7, PaneKind::Chart(2));
        terminal
            .detached_chart_windows
            .insert(iced::window::Id::unique(), DetachedChartWindowState::new(3));
        terminal
            .spaghetti_charts
            .insert(4, SpaghettiChartInstance::new_pair(4));
        let mut anchored = SpaghettiChartInstance::new_empty(5);
        anchored.canvas.active_session = Some(crate::spaghetti::Session::UtcDay);
        terminal.spaghetti_charts.insert(5, anchored);
        terminal.status_bar_now_ms = TradingTerminal::now_ms();
        assert_eq!(terminal.common_chart_timeframe(), None);

        let _task = terminal.update(Message::AllChartsSwitchTimeframe(Timeframe::H4));

        for instance in terminal.charts.values() {
            assert_eq!(instance.interval, Timeframe::H4);
            assert!(!instance.chart.hud_armed());
            assert_eq!(
                instance.candle_fetch_request.as_ref().map(|r| r.timeframe),
                Some(Timeframe::H4),
            );
        }
        assert_eq!(terminal.spaghetti_charts[&4].interval, Timeframe::H4);
        assert_eq!(
            terminal.spaghetti_charts[&5].canvas.active_session,
            Some(crate::spaghetti::Session::UtcDay),
        );
        assert_eq!(
            terminal.spaghetti_charts[&5].session_granularity,
            Some(Timeframe::H4),
        );
        assert_eq!(terminal.common_chart_timeframe(), Some(Timeframe::H4));
        assert!(terminal.config_save_due_at.is_some());
        assert!(
            terminal
                .chart_configs_snapshot()
                .iter()
                .all(|config| { config.timeframe == Timeframe::H4.config_str() })
        );

        let _task = terminal.update(Message::ChartSwitchTimeframe(2, Timeframe::M15));
        assert_eq!(terminal.charts[&2].interval, Timeframe::M15);
        assert_eq!(terminal.charts[&1].interval, Timeframe::H4);
        assert_eq!(terminal.charts[&3].interval, Timeframe::H4);
        assert_eq!(terminal.spaghetti_charts[&4].interval, Timeframe::H4);
        assert_eq!(terminal.common_chart_timeframe(), None);

        let _task = terminal.update(Message::AllChartsSwitchTimeframe(Timeframe::D1));
        assert_eq!(terminal.common_chart_timeframe(), Some(Timeframe::D1));
    }

    #[test]
    fn global_timeframe_replaces_primary_and_secondary_requests_and_rejects_old_results() {
        let mut terminal = empty_terminal();
        let mut instance = ChartInstance::new(1, "BTC".into(), Timeframe::H1);
        instance.set_secondary_symbol_identity("ETH".into(), "ETH".into());
        terminal.charts.insert(1, instance);
        let _task = terminal.update(Message::ChartSwitchTimeframe(1, Timeframe::M5));
        let old_primary = terminal.charts[&1]
            .candle_fetch_request
            .clone()
            .expect("primary request");
        let old_secondary = terminal.charts[&1]
            .secondary_candle_fetch_request
            .clone()
            .expect("secondary request");

        let _task = terminal.update(Message::AllChartsSwitchTimeframe(Timeframe::H4));
        let primary = terminal.charts[&1].candle_fetch_request.clone();
        let secondary = terminal.charts[&1].secondary_candle_fetch_request.clone();
        assert_eq!(primary.as_ref().map(|r| r.timeframe), Some(Timeframe::H4));
        assert_eq!(secondary.as_ref().map(|r| r.timeframe), Some(Timeframe::H4));

        let _task = terminal.update(Message::ChartCandlesLoaded(
            old_primary,
            Err("old timeframe failure".into()),
        ));
        let _task = terminal.update(Message::ChartSecondaryCandlesLoaded(
            old_secondary,
            Err("old timeframe failure".into()),
        ));
        let instance = &terminal.charts[&1];
        assert_eq!(instance.candle_fetch_request, primary);
        assert_eq!(instance.secondary_candle_fetch_request, secondary);
        assert!(instance.candle_fetch_error.is_none());
        assert!(instance.secondary_candle_fetch_error.is_none());
    }

    #[test]
    fn global_timeframe_preserves_loaded_charts_already_at_the_requested_interval() {
        let mut terminal = empty_terminal();
        let mut instance = ChartInstance::new(1, "BTC".into(), Timeframe::H4);
        instance.chart.set_candles(vec![Candle::test_ohlcv(
            Timeframe::H4.duration_ms(),
            2 * Timeframe::H4.duration_ms(),
            [100.0, 101.0, 99.0, 100.0],
            1.0,
        )]);
        assert_eq!(instance.chart.candles.len(), 1);
        instance.chart.status = ChartStatus::Loaded;
        terminal.charts.insert(1, instance);

        let _task = terminal.update(Message::AllChartsSwitchTimeframe(Timeframe::H4));

        assert_eq!(terminal.charts[&1].chart.candles.len(), 1);
        assert!(matches!(
            terminal.charts[&1].chart.status,
            ChartStatus::Loaded
        ));
        assert!(terminal.charts[&1].candle_fetch_request.is_none());
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn global_timeframe_with_no_charts_is_a_noop() {
        let mut terminal = empty_terminal();

        let _task = terminal.update(Message::AllChartsSwitchTimeframe(Timeframe::H4));

        assert!(!terminal.has_open_timeframe_charts());
        assert_eq!(terminal.common_chart_timeframe(), None);
        assert!(terminal.config_save_due_at.is_none());
    }

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
