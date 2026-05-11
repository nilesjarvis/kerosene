use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::config;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::Timeframe;
use iced::{Task, Theme};

// ---------------------------------------------------------------------------
// Layout Chart Instance Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_chart_instances(
        &mut self,
        chart_configs: &[config::ChartConfig],
        spaghetti_configs: &[config::SpaghettiChartConfig],
        next_chart_id: ChartId,
        next_spaghetti_id: SpaghettiChartId,
    ) -> Vec<Task<Message>> {
        let mut boot_tasks = Vec::new();
        self.restore_saved_chart_instances(chart_configs, next_chart_id, &mut boot_tasks);
        self.restore_saved_spaghetti_instances(
            spaghetti_configs,
            next_spaghetti_id,
            &mut boot_tasks,
        );
        boot_tasks
    }

    fn restore_saved_chart_instances(
        &mut self,
        chart_configs: &[config::ChartConfig],
        next_chart_id: ChartId,
        boot_tasks: &mut Vec<Task<Message>>,
    ) {
        let mut charts = std::collections::HashMap::new();
        for chart_cfg in chart_configs {
            let id = chart_cfg.id;
            let tf = Timeframe::from_config_str(&chart_cfg.timeframe);
            let mut instance = ChartInstance::new(id, chart_cfg.symbol.clone(), tf);
            instance.chart.inverted = chart_cfg.inverted;
            instance
                .chart
                .set_funding_panel_height(chart_cfg.funding_panel_height as f32);
            instance.macro_indicators = chart_cfg.macro_indicators.clone();
            instance.chart.macro_indicators = chart_cfg.macro_indicators.clone();
            let mut ann_id: AnnotationId = 0;
            for acfg in &chart_cfg.annotations {
                if let Some(ann) = Annotation::from_config(ann_id, acfg) {
                    instance.annotations.push(ann);
                    ann_id += 1;
                }
            }
            instance.next_annotation_id = ann_id;
            instance.chart.annotations = instance.annotations.clone();
            if !chart_cfg.symbol.is_empty() && !self.is_ticker_muted(&chart_cfg.symbol) {
                let request = Self::build_candle_fetch_request(id, &chart_cfg.symbol, tf, None, 0);
                instance.candle_fetch_request = Some(request.clone());
                boot_tasks.push(Self::fetch_candles_task(request));
                boot_tasks.extend(Self::fetch_macro_candles_tasks(id, &chart_cfg.symbol));
            } else if !chart_cfg.symbol.is_empty() {
                Self::clear_chart_for_muted_symbol(&mut instance);
            }
            charts.insert(id, instance);
        }
        self.charts = charts;
        self.next_chart_id = next_chart_id;
    }

    fn restore_saved_spaghetti_instances(
        &mut self,
        spaghetti_configs: &[config::SpaghettiChartConfig],
        next_spaghetti_id: SpaghettiChartId,
        boot_tasks: &mut Vec<Task<Message>>,
    ) {
        let mut spaghetti_charts = std::collections::HashMap::new();
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
                .filter(|sym_key| !self.is_ticker_muted(sym_key))
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
        self.spaghetti_charts = spaghetti_charts;
        self.next_spaghetti_id = next_spaghetti_id;
    }
}
