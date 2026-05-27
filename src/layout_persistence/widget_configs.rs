use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::config;
use crate::spaghetti_state::SpaghettiChartId;

pub(crate) struct LayoutWidgetConfigs {
    pub(crate) chart_configs: Vec<config::ChartConfig>,
    pub(crate) spaghetti_configs: Vec<config::SpaghettiChartConfig>,
    pub(crate) next_chart_id: ChartId,
    pub(crate) next_spaghetti_id: SpaghettiChartId,
}

impl TradingTerminal {
    pub(crate) fn normalized_layout_widget_configs(
        &self,
        layout: &config::SavedLayout,
    ) -> LayoutWidgetConfigs {
        let mut chart_configs = if layout.charts.is_empty() {
            vec![config::ChartConfig::empty(
                0,
                self.active_symbol.clone(),
                layout.active_timeframe.clone(),
            )]
        } else {
            layout.charts.clone()
        };

        if chart_configs.is_empty() {
            chart_configs.push(config::ChartConfig::empty(
                0,
                self.active_symbol.clone(),
                layout.active_timeframe.clone(),
            ));
        }

        let mut used_chart_ids = std::collections::BTreeSet::new();
        let mut next_chart_id: ChartId = 0;
        for chart_cfg in &mut chart_configs {
            if used_chart_ids.contains(&chart_cfg.id) {
                while used_chart_ids.contains(&next_chart_id) {
                    next_chart_id += 1;
                }
                chart_cfg.id = next_chart_id;
            }
            used_chart_ids.insert(chart_cfg.id);
            next_chart_id = next_chart_id.max(chart_cfg.id.saturating_add(1));
        }

        let mut spaghetti_configs = layout.spaghetti_charts.clone();
        let mut used_spaghetti_ids = std::collections::BTreeSet::new();
        let mut next_spaghetti_id: SpaghettiChartId = 0;
        for spaghetti_cfg in &mut spaghetti_configs {
            if used_spaghetti_ids.contains(&spaghetti_cfg.id) {
                while used_spaghetti_ids.contains(&next_spaghetti_id) {
                    next_spaghetti_id += 1;
                }
                spaghetti_cfg.id = next_spaghetti_id;
            }
            used_spaghetti_ids.insert(spaghetti_cfg.id);
            next_spaghetti_id = next_spaghetti_id.max(spaghetti_cfg.id.saturating_add(1));
        }

        if let Some(pane_layout) = &layout.pane_layout {
            let mut layout_chart_ids = std::collections::BTreeSet::new();
            let mut layout_spaghetti_ids = std::collections::BTreeSet::new();
            Self::collect_layout_widget_ids(
                pane_layout,
                &mut layout_chart_ids,
                &mut layout_spaghetti_ids,
            );

            for id in layout_chart_ids {
                if used_chart_ids.insert(id) {
                    chart_configs.push(config::ChartConfig::empty(id, String::new(), "H1"));
                    next_chart_id = next_chart_id.max(id.saturating_add(1));
                }
            }

            for id in layout_spaghetti_ids {
                if used_spaghetti_ids.insert(id) {
                    spaghetti_configs.push(config::SpaghettiChartConfig::empty(id));
                    next_spaghetti_id = next_spaghetti_id.max(id.saturating_add(1));
                }
            }
        }

        chart_configs.sort_by_key(|chart| chart.id);
        spaghetti_configs.sort_by_key(|spaghetti| spaghetti.id);

        LayoutWidgetConfigs {
            chart_configs,
            spaghetti_configs,
            next_chart_id,
            next_spaghetti_id,
        }
    }
}
