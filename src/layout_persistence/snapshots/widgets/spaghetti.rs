use crate::app_state::TradingTerminal;
use crate::config;

impl TradingTerminal {
    pub(crate) fn spaghetti_chart_configs_snapshot(&self) -> Vec<config::SpaghettiChartConfig> {
        let mut spaghetti_instances: Vec<_> = self.spaghetti_charts.values().collect();
        spaghetti_instances.sort_by_key(|inst| inst.id);
        spaghetti_instances
            .into_iter()
            .map(|inst| config::SpaghettiChartConfig {
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
            })
            .collect()
    }
}
