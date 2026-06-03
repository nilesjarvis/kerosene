use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::config;

impl TradingTerminal {
    pub(crate) fn chart_configs_snapshot(&self) -> Vec<config::ChartConfig> {
        let mut chart_instances: Vec<_> = self.charts.values().collect();
        chart_instances.sort_by_key(|inst| inst.id);
        chart_instances
            .into_iter()
            .map(|inst| self.chart_config_for_instance(inst))
            .collect()
    }

    pub(crate) fn docked_chart_configs_snapshot(&self) -> Vec<config::ChartConfig> {
        let mut chart_instances: Vec<_> = self
            .charts
            .values()
            .filter(|inst| self.chart_is_docked(inst.id))
            .collect();
        chart_instances.sort_by_key(|inst| inst.id);
        chart_instances
            .into_iter()
            .map(|inst| self.chart_config_for_instance(inst))
            .collect()
    }

    fn chart_config_for_instance(&self, inst: &ChartInstance) -> config::ChartConfig {
        config::ChartConfig {
            id: inst.id,
            symbol: if self.symbol_key_is_hidden(&inst.symbol) {
                String::new()
            } else {
                inst.symbol.clone()
            },
            timeframe: inst.interval.config_str().to_string(),
            annotations: inst
                .annotations
                .iter()
                .filter(|annotation| annotation.is_valid())
                .map(|annotation| annotation.to_config())
                .collect(),
            inverted: inst.chart.inverted,
            show_trade_markers: inst.chart.show_trade_markers,
            show_earnings_markers: inst.show_earnings_markers,
            header_collapsed: inst.header_collapsed,
            funding_panel_height: inst.chart.funding_panel_height_config(),
            macro_indicators: inst.macro_indicators.clone(),
            open_interest_as_notional: inst.open_interest_as_notional,
            asset_volume_as_notional: inst.asset_volume_as_notional,
            outcome_volume_as_notional: inst.outcome_volume_as_notional,
        }
    }

    pub(crate) fn detached_chart_window_configs_snapshot(
        &self,
    ) -> Vec<config::DetachedChartWindowConfig> {
        let mut windows: Vec<_> = self.detached_chart_windows.values().collect();
        windows.sort_by_key(|state| state.chart_id);
        windows
            .into_iter()
            .filter(|state| self.charts.contains_key(&state.chart_id))
            .map(|state| state.to_config())
            .collect()
    }
}
