use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::config::{ChartConfig, SpaghettiChartConfig};
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::Timeframe;
use iced::{Task, Theme};
use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    pub(crate) fn boot_chart_instances(
        chart_configs: &[ChartConfig],
        muted_tickers: &HashSet<String>,
    ) -> (HashMap<ChartId, ChartInstance>, Vec<Task<Message>>) {
        let mut boot_tasks = Vec::new();
        let mut charts = HashMap::new();

        for chart_cfg in chart_configs {
            let id = chart_cfg.id;
            let tf = Timeframe::from_config_str(&chart_cfg.timeframe);
            let mut instance = ChartInstance::new(id, chart_cfg.symbol.clone(), tf);
            instance.chart.inverted = chart_cfg.inverted;
            instance.chart.show_trade_markers = chart_cfg.show_trade_markers;
            instance
                .chart
                .set_funding_panel_height(chart_cfg.funding_panel_height as f32);
            instance.macro_indicators = chart_cfg.macro_indicators.clone();
            instance.chart.macro_indicators = chart_cfg.macro_indicators.clone();
            instance.open_interest_as_notional = chart_cfg.open_interest_as_notional;

            let mut ann_id: AnnotationId = 0;
            for acfg in &chart_cfg.annotations {
                if let Some(ann) = Annotation::from_config(ann_id, acfg) {
                    instance.annotations.push(ann);
                    ann_id += 1;
                }
            }
            instance.next_annotation_id = ann_id;
            instance.chart.annotations = instance.annotations.clone();

            if !chart_cfg.symbol.is_empty()
                && !Self::key_matches_muted_tickers(&[], muted_tickers, &chart_cfg.symbol)
            {
                let request = Self::build_candle_fetch_request(id, &chart_cfg.symbol, tf, None, 0);
                instance.candle_fetch_request = Some(request.clone());
                boot_tasks.push(Self::fetch_candles_task(request));
                boot_tasks.extend(Self::fetch_macro_candles_tasks(id, &chart_cfg.symbol));
            } else if !chart_cfg.symbol.is_empty() {
                Self::clear_chart_for_muted_symbol(&mut instance);
            }

            charts.insert(id, instance);
        }

        (charts, boot_tasks)
    }

    pub(crate) fn boot_spaghetti_instances(
        spaghetti_configs: &[SpaghettiChartConfig],
        muted_tickers: &HashSet<String>,
    ) -> (
        HashMap<SpaghettiChartId, SpaghettiChartInstance>,
        Vec<Task<Message>>,
    ) {
        let mut boot_tasks = Vec::new();
        let mut spaghetti_charts = HashMap::new();

        for scfg in spaghetti_configs {
            let sid = scfg.id;
            let tf = Timeframe::from_config_str(&scfg.timeframe);
            let mut inst = SpaghettiChartInstance::new_empty(sid);
            inst.interval = tf;
            inst.pair_mode = scfg.pair_mode;
            inst.canvas.pair_ratio_mode = scfg.pair_mode;
            inst.pair_candle_mode = scfg.pair_candle_mode;
            inst.canvas.pair_candle_mode = scfg.pair_candle_mode;
            inst.canvas.color_mode = scfg.color_mode;
            inst.canvas.show_labels = scfg.show_labels;
            inst.canvas.active_session = scfg
                .anchor
                .as_deref()
                .and_then(spaghetti::Session::from_config_str);
            inst.session_granularity = scfg
                .anchor_granularity
                .as_deref()
                .and_then(Timeframe::from_config_str_opt);
            Self::normalize_spaghetti_session_granularity(&mut inst, Self::now_ms());
            inst.editor_open = false;

            for sym_key in scfg
                .symbols
                .iter()
                .filter(|sym_key| !Self::key_matches_muted_tickers(&[], muted_tickers, sym_key))
            {
                let color_idx = inst.next_color_idx;
                inst.next_color_idx += 1;
                let colors = spaghetti::series_colors(&Theme::Dark);
                let color = colors[color_idx % colors.len()];
                let display = sym_key.split(':').nth(1).unwrap_or(sym_key).to_string();
                inst.canvas.series.push(spaghetti::Series {
                    symbol: sym_key.clone(),
                    display,
                    candles: Vec::new(),
                    color,
                    loaded: false,
                });
                boot_tasks.push(Self::fetch_spaghetti_candles(
                    sid,
                    sym_key,
                    tf,
                    inst.canvas.active_session,
                    inst.session_granularity,
                    None,
                ));
            }

            spaghetti_charts.insert(sid, inst);
        }

        (spaghetti_charts, boot_tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MacroIndicatorsConfig;

    #[test]
    fn boot_chart_instances_restores_trade_marker_toggle() {
        let configs = vec![ChartConfig {
            id: 3,
            symbol: String::new(),
            timeframe: "H1".to_string(),
            annotations: Vec::new(),
            inverted: false,
            show_trade_markers: true,
            funding_panel_height: 56,
            macro_indicators: MacroIndicatorsConfig::default(),
            open_interest_as_notional: false,
        }];

        let (charts, tasks) =
            TradingTerminal::boot_chart_instances(&configs, &std::collections::HashSet::new());

        assert!(tasks.is_empty());
        assert!(
            charts
                .get(&3)
                .expect("chart instance")
                .chart
                .show_trade_markers
        );
    }
}
