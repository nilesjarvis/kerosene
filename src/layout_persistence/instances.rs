use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartBackfillFetchContext, ChartId, ChartInstance};
use crate::config;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
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
        self.spaghetti_instance_epoch = self.spaghetti_instance_epoch.wrapping_add(1);
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
            let primary_symbol = self
                .exchange_symbol_for_key(&chart_cfg.symbol)
                .map(|metadata| metadata.key.clone())
                .unwrap_or_else(|| chart_cfg.symbol.clone());
            let primary_alias_canonicalized = primary_symbol != chart_cfg.symbol;
            let primary_legacy_spot_unresolved = primary_symbol == "@0";
            let mut instance = ChartInstance::from_config(chart_cfg, primary_symbol.clone());
            let tf = instance.interval;
            // Layouts restore at runtime too, when no SymbolsLoaded message
            // will arrive to repair the raw-key placeholder from
            // ChartInstance::new; resolve the display name here.
            let display = self.display_name_for_symbol(&primary_symbol);
            instance.set_symbol_identity(primary_symbol.clone(), display);
            if let Some(requested_secondary) = chart_cfg
                .secondary_symbol
                .as_ref()
                .filter(|symbol| !symbol.is_empty())
            {
                let secondary_symbol = self
                    .exchange_symbol_for_key(requested_secondary)
                    .map(|metadata| metadata.key.clone())
                    .unwrap_or_else(|| requested_secondary.clone());
                let secondary_alias_canonicalized =
                    secondary_symbol.as_str() != requested_secondary.as_str();
                let alias_collision = secondary_symbol == primary_symbol
                    && (primary_alias_canonicalized || secondary_alias_canonicalized);
                if !alias_collision && !self.is_ticker_muted(&secondary_symbol) {
                    let display = self.display_name_for_symbol(&secondary_symbol);
                    instance.set_secondary_symbol_identity(secondary_symbol, display);
                }
            }
            if !primary_symbol.is_empty() && !self.symbol_key_is_hidden(&primary_symbol) {
                if tf.uses_candle_backfill() && !primary_legacy_spot_unresolved {
                    let request = Self::build_candle_fetch_request(
                        id,
                        &primary_symbol,
                        tf,
                        self.chart_backfill_request_context_for_timeframe(tf),
                        None,
                        0,
                    );
                    instance.candle_fetch_request = Some(request.clone());
                    boot_tasks.push(Self::fetch_candles_task(
                        request,
                        self.hydromancer_api_key_for_task(),
                    ));
                } else if !tf.uses_candle_backfill() {
                    instance.chart.status = crate::chart::ChartStatus::Loaded;
                }
                if !primary_legacy_spot_unresolved {
                    let macro_request_id = instance.next_macro_candles_request_id();
                    boot_tasks.extend(Self::fetch_macro_candles_tasks(
                        id,
                        macro_request_id,
                        &primary_symbol,
                        &instance.macro_indicators,
                    ));
                }
            } else if !primary_symbol.is_empty() && self.is_ticker_muted(&primary_symbol) {
                Self::clear_chart_for_muted_symbol(&mut instance);
            }
            if let Some(symbol) = instance.secondary_symbol.clone()
                && tf.uses_candle_backfill()
                && symbol != "@0"
                && !self.symbol_key_is_hidden(&symbol)
            {
                let request = Self::build_candle_fetch_request(
                    id,
                    &symbol,
                    tf,
                    self.chart_backfill_request_context_for_timeframe(tf),
                    None,
                    0,
                );
                instance.secondary_candle_fetch_request = Some(request.clone());
                boot_tasks.push(Self::fetch_secondary_candles_task(
                    request,
                    self.hydromancer_api_key_for_task(),
                ));
            }
            charts.insert(id, instance);
        }
        self.charts = charts;
        self.next_chart_id = next_chart_id;
        boot_tasks.push(self.refresh_enabled_earnings_charts());
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
            let mut inst = SpaghettiChartInstance::from_config(scfg);
            let tf = inst.interval;
            Self::normalize_spaghetti_session_granularity(&mut inst, Self::now_ms());
            let mut seen_symbols = std::collections::HashSet::new();

            for sym_key in scfg
                .symbols
                .iter()
                .filter(|sym_key| !self.is_ticker_muted(sym_key))
            {
                let canonical_key = self
                    .exchange_symbol_for_key(sym_key)
                    .map(|metadata| metadata.key.clone())
                    .unwrap_or_else(|| sym_key.clone());
                if !seen_symbols.insert(canonical_key.clone()) {
                    continue;
                }
                let color_idx = inst.next_color_idx;
                inst.next_color_idx += 1;
                let colors = spaghetti::series_colors(&Theme::Dark);
                let color = colors[color_idx % colors.len()];
                let display = self.display_name_for_symbol(&canonical_key);
                inst.canvas.series.push(spaghetti::Series {
                    symbol: canonical_key.clone(),
                    display,
                    candles: Vec::new(),
                    color,
                    loaded: false,
                });
                if !self.symbol_key_is_hidden(&canonical_key) {
                    boot_tasks.push(Self::fetch_spaghetti_candles(
                        sid,
                        self.spaghetti_instance_epoch,
                        &canonical_key,
                        tf,
                        inst.canvas.active_session,
                        inst.session_granularity,
                        ChartBackfillFetchContext::new(
                            self.chart_backfill_source,
                            self.read_data_provider_generation,
                            self.hydromancer_key_generation,
                            self.hydromancer_api_key_for_task(),
                        ),
                    ));
                }
            }

            spaghetti_charts.insert(sid, inst);
        }
        self.spaghetti_charts = spaghetti_charts;
        self.next_spaghetti_id = next_spaghetti_id;
    }
}

#[cfg(test)]
mod tests;
