use crate::app_state::TradingTerminal;
use crate::config;

impl TradingTerminal {
    pub(crate) fn spaghetti_chart_configs_snapshot(&self) -> Vec<config::SpaghettiChartConfig> {
        let mut spaghetti_instances: Vec<_> = self.spaghetti_charts.values().collect();
        spaghetti_instances.sort_by_key(|inst| inst.id);
        spaghetti_instances
            .into_iter()
            .map(|inst| self.spaghetti_config_for_instance(inst))
            .collect()
    }

    pub(crate) fn docked_spaghetti_chart_configs_snapshot(
        &self,
    ) -> Vec<config::SpaghettiChartConfig> {
        let mut spaghetti_instances: Vec<_> = self
            .spaghetti_charts
            .values()
            .filter(|inst| self.spaghetti_is_docked(inst.id))
            .collect();
        spaghetti_instances.sort_by_key(|inst| inst.id);
        spaghetti_instances
            .into_iter()
            .map(|inst| self.spaghetti_config_for_instance(inst))
            .collect()
    }

    fn spaghetti_config_for_instance(
        &self,
        inst: &crate::spaghetti_state::SpaghettiChartInstance,
    ) -> config::SpaghettiChartConfig {
        config::SpaghettiChartConfig {
            id: inst.id,
            symbols: inst
                .canvas
                .series
                .iter()
                .filter(|series| !self.symbol_key_is_hidden(&series.symbol))
                .map(|series| series.symbol.clone())
                .collect(),
            timeframe: inst.interval.config_str().to_string(),
            pair_mode: inst.pair_mode,
            pair_candle_mode: inst.pair_candle_mode,
            color_mode: inst.canvas.color_mode,
            show_labels: inst.canvas.show_labels,
            anchor: inst
                .canvas
                .active_session
                .map(|session| session.config_str().to_string()),
            anchor_granularity: inst
                .session_granularity
                .map(|granularity| granularity.config_str().to_string()),
        }
    }

    pub(crate) fn detached_spaghetti_window_configs_snapshot(
        &self,
    ) -> Vec<config::DetachedSpaghettiWindowConfig> {
        let mut windows: Vec<_> = self.detached_spaghetti_windows.values().collect();
        windows.sort_by_key(|state| state.chart_id);
        windows
            .into_iter()
            .filter(|state| self.spaghetti_charts.contains_key(&state.chart_id))
            .map(|state| state.to_config())
            .collect()
    }
}
