use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::config::{ChartConfig, KeroseneConfig, SpaghettiChartConfig};
use crate::layout_persistence::LayoutWidgetConfigs;
use crate::spaghetti_state::SpaghettiChartId;
use std::collections::BTreeSet;

impl TradingTerminal {
    pub(crate) fn boot_layout_widget_configs(
        cfg: &KeroseneConfig,
        active_symbol: &str,
    ) -> LayoutWidgetConfigs {
        let mut chart_configs = if cfg.charts.is_empty() {
            vec![ChartConfig::empty(
                0,
                active_symbol,
                cfg.active_timeframe.clone(),
            )]
        } else {
            cfg.charts.clone()
        };

        if chart_configs.is_empty() {
            chart_configs.push(ChartConfig::empty(
                0,
                active_symbol,
                cfg.active_timeframe.clone(),
            ));
        }

        let mut used_chart_ids = BTreeSet::new();
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

        let mut spaghetti_configs = cfg.spaghetti_charts.clone();
        let mut used_spaghetti_ids = BTreeSet::new();
        let mut next_spaghetti_id: SpaghettiChartId = 0;
        for scfg in &mut spaghetti_configs {
            if used_spaghetti_ids.contains(&scfg.id) {
                while used_spaghetti_ids.contains(&next_spaghetti_id) {
                    next_spaghetti_id += 1;
                }
                scfg.id = next_spaghetti_id;
            }
            used_spaghetti_ids.insert(scfg.id);
            next_spaghetti_id = next_spaghetti_id.max(scfg.id.saturating_add(1));
        }

        if let Some(layout) = &cfg.pane_layout {
            let mut layout_chart_ids = BTreeSet::new();
            let mut layout_spaghetti_ids = BTreeSet::new();
            Self::collect_layout_widget_ids(
                layout,
                &mut layout_chart_ids,
                &mut layout_spaghetti_ids,
            );

            for id in layout_chart_ids {
                if used_chart_ids.insert(id) {
                    chart_configs.push(ChartConfig::empty(id, String::new(), "H1"));
                    next_chart_id = next_chart_id.max(id.saturating_add(1));
                }
            }

            for id in layout_spaghetti_ids {
                if used_spaghetti_ids.insert(id) {
                    spaghetti_configs.push(SpaghettiChartConfig::empty(id));
                    next_spaghetti_id = next_spaghetti_id.max(id.saturating_add(1));
                }
            }
        }

        chart_configs.sort_by_key(|config| config.id);
        spaghetti_configs.sort_by_key(|config| config.id);

        LayoutWidgetConfigs {
            chart_configs,
            spaghetti_configs,
            next_chart_id,
            next_spaghetti_id,
        }
    }
}
