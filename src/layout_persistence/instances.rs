use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartBackfillFetchContext, ChartId, ChartInstance};
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
        self.chart_instance_generation = self.chart_instance_generation.wrapping_add(1);
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
            let primary_symbol = self
                .exchange_symbol_for_key(&chart_cfg.symbol)
                .map(|metadata| metadata.key.clone())
                .unwrap_or_else(|| chart_cfg.symbol.clone());
            let primary_alias_canonicalized = primary_symbol != chart_cfg.symbol;
            let primary_legacy_spot_unresolved = primary_symbol == "@0";
            let mut instance = ChartInstance::new(id, primary_symbol.clone(), tf);
            // Layouts restore at runtime too, when no SymbolsLoaded message
            // will arrive to repair the raw-key placeholder from
            // ChartInstance::new; resolve the display name here.
            let display = self.display_name_for_symbol(&primary_symbol);
            instance.set_symbol_identity(primary_symbol.clone(), display);
            instance.chart.inverted = chart_cfg.inverted;
            instance.chart.show_trade_markers = chart_cfg.show_trade_markers;
            instance.show_earnings_markers = chart_cfg.show_earnings_markers;
            instance.header_collapsed = chart_cfg.header_collapsed;
            instance.drawing_toolbar_collapsed = chart_cfg.drawing_toolbar_collapsed;
            instance
                .chart
                .set_funding_panel_height(chart_cfg.funding_panel_height as f32);
            instance
                .chart
                .set_session_panel_height(chart_cfg.session_panel_height as f32);
            instance.macro_indicators = chart_cfg.macro_indicators.clone();
            instance.chart.macro_indicators = chart_cfg.macro_indicators.clone();
            instance.open_interest_as_notional = chart_cfg.open_interest_as_notional;
            instance.asset_volume_as_notional = chart_cfg.asset_volume_as_notional;
            instance.outcome_volume_as_notional = chart_cfg.outcome_volume_as_notional;
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
                if !alias_collision && !self.symbol_key_is_hidden(&secondary_symbol) {
                    let display = self.display_name_for_symbol(&secondary_symbol);
                    instance.set_secondary_symbol_identity(secondary_symbol, display);
                }
            }
            let mut ann_id: AnnotationId = 0;
            for acfg in &chart_cfg.annotations {
                if let Some(ann) = Annotation::from_config(ann_id, acfg) {
                    instance.annotations.push(ann);
                    ann_id += 1;
                }
            }
            instance.next_annotation_id = ann_id;
            instance.chart.annotations = instance.annotations.clone();
            if !primary_symbol.is_empty() && !self.symbol_key_is_hidden(&primary_symbol) {
                if tf.uses_candle_backfill() && !primary_legacy_spot_unresolved {
                    let request = Self::build_candle_fetch_request(
                        id,
                        &primary_symbol,
                        tf,
                        self.chart_backfill_request_context_for_symbol_timeframe(
                            &primary_symbol,
                            tf,
                        ),
                        None,
                        0,
                    );
                    instance.candle_fetch_request = Some(request.clone());
                    boot_tasks.push(Self::fetch_candles_task(
                        request,
                        self.hydromancer_api_key_for_task(),
                        self.schwab.access_token_for_task(),
                    ));
                } else if !tf.uses_candle_backfill() {
                    instance.chart.status = crate::chart::ChartStatus::Loaded;
                }
                if !primary_legacy_spot_unresolved {
                    let macro_request_id = instance.next_macro_candles_request_id();
                    boot_tasks.extend(Self::fetch_macro_candles_tasks(
                        id,
                        self.chart_instance_generation,
                        macro_request_id,
                        &primary_symbol,
                    ));
                }
            } else if !primary_symbol.is_empty() {
                Self::clear_chart_for_muted_symbol(&mut instance);
            }
            if let Some(symbol) = instance.secondary_symbol.clone()
                && tf.uses_candle_backfill()
                && symbol != "@0"
            {
                let request = Self::build_candle_fetch_request(
                    id,
                    &symbol,
                    tf,
                    self.chart_backfill_request_context_for_symbol_timeframe(&symbol, tf),
                    None,
                    0,
                );
                instance.secondary_candle_fetch_request = Some(request.clone());
                boot_tasks.push(Self::fetch_secondary_candles_task(
                    request,
                    self.hydromancer_api_key_for_task(),
                    self.schwab.access_token_for_task(),
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
            let mut seen_symbols = std::collections::HashSet::new();

            for sym_key in scfg
                .symbols
                .iter()
                .filter(|sym_key| !self.symbol_key_is_hidden(sym_key))
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
                boot_tasks.push(Self::queue_spaghetti_candle_fetch(
                    &mut inst,
                    &canonical_key,
                    self.chart_instance_generation,
                    None,
                    ChartBackfillFetchContext::new(
                        self.chart_backfill_source,
                        self.read_data_provider_generation,
                        self.hydromancer_key_generation,
                        self.hydromancer_api_key_for_task(),
                    ),
                ));
            }

            spaghetti_charts.insert(sid, inst);
        }
        self.spaghetti_charts = spaghetti_charts;
        self.next_spaghetti_id = next_spaghetti_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType, USDC_TOKEN_INDEX};

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "test".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 4,
            max_leverage: 1,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn canonical_purr_symbol() -> ExchangeSymbol {
        ExchangeSymbol {
            ticker: "PURR".to_string(),
            category: "spot".to_string(),
            display_name: Some("PURR/USDC".to_string()),
            asset_index: 10_000,
            collateral_token: Some(USDC_TOKEN_INDEX),
            sz_decimals: 0,
            ..symbol("PURR/USDC", MarketType::Spot)
        }
    }

    #[test]
    fn runtime_layout_restore_canonicalizes_regular_chart_spot_aliases_before_fetch() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![
            symbol("BTC", MarketType::Perp),
            symbol("ETH", MarketType::Perp),
            canonical_purr_symbol(),
        ];

        let mut legacy_primary = config::ChartConfig::empty(1, "@0", "H1");
        legacy_primary.secondary_symbol = Some("ETH".to_string());
        let mut legacy_secondary = config::ChartConfig::empty(2, "BTC", "H1");
        legacy_secondary.secondary_symbol = Some("@0".to_string());
        let mut alias_collision = config::ChartConfig::empty(3, "PURR/USDC", "H1");
        alias_collision.secondary_symbol = Some("@0".to_string());
        let mut unchanged_perps = config::ChartConfig::empty(4, "BTC", "H1");
        unchanged_perps.secondary_symbol = Some("ETH".to_string());

        let _tasks = terminal.restore_layout_chart_instances(
            &[
                legacy_primary,
                legacy_secondary,
                alias_collision,
                unchanged_perps,
            ],
            &[],
            5,
            0,
        );

        assert_eq!(terminal.chart_instance_generation, 1);

        let primary = &terminal.charts[&1];
        assert_eq!(primary.symbol, "PURR/USDC");
        assert_eq!(primary.symbol_display, "PURR/USDC");
        assert_eq!(
            primary
                .candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str()),
            Some("PURR/USDC")
        );
        assert_eq!(primary.secondary_symbol.as_deref(), Some("ETH"));

        let secondary = &terminal.charts[&2];
        assert_eq!(secondary.symbol, "BTC");
        assert_eq!(secondary.secondary_symbol.as_deref(), Some("PURR/USDC"));
        assert_eq!(
            secondary
                .secondary_candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str()),
            Some("PURR/USDC")
        );

        let deduplicated = &terminal.charts[&3];
        assert_eq!(deduplicated.symbol, "PURR/USDC");
        assert!(deduplicated.secondary_symbol.is_none());
        assert!(deduplicated.secondary_candle_fetch_request.is_none());

        let perps = &terminal.charts[&4];
        assert_eq!(perps.symbol, "BTC");
        assert_eq!(perps.secondary_symbol.as_deref(), Some("ETH"));
        assert_eq!(
            perps
                .candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str()),
            Some("BTC")
        );
        assert_eq!(
            perps
                .secondary_candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str()),
            Some("ETH")
        );

        assert!(terminal.charts.values().all(|chart| {
            chart
                .candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str())
                != Some("@0")
                && chart
                    .secondary_candle_fetch_request
                    .as_ref()
                    .map(|request| request.symbol.as_str())
                    != Some("@0")
        }));
        assert!(terminal.charts.values().all(|chart| {
            chart
                .candle_fetch_request
                .as_ref()
                .is_none_or(|request| request.chart_instance_generation == 1)
                && chart
                    .secondary_candle_fetch_request
                    .as_ref()
                    .is_none_or(|request| request.chart_instance_generation == 1)
        }));
    }

    #[test]
    fn repeated_runtime_layout_restore_advances_chart_incarnation() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
        terminal.chart_instance_generation = 41;
        let config = config::ChartConfig::empty(7, "BTC", "H1");
        let mut spaghetti_config = config::SpaghettiChartConfig::empty(9);
        spaghetti_config.symbols.push("BTC".to_string());

        let _tasks = terminal.restore_layout_chart_instances(
            &[config.clone()],
            &[spaghetti_config.clone()],
            8,
            10,
        );

        assert_eq!(terminal.chart_instance_generation, 42);
        assert_eq!(
            terminal.charts[&7]
                .candle_fetch_request
                .as_ref()
                .map(|request| request.chart_instance_generation),
            Some(42)
        );
        assert_eq!(terminal.charts[&7].macro_candles_request_id, 1);
        assert_eq!(
            terminal.spaghetti_charts[&9].pending_spaghetti_candle_request_id("BTC"),
            Some(1)
        );

        let _tasks = terminal.restore_layout_chart_instances(&[config], &[spaghetti_config], 8, 10);

        assert_eq!(terminal.chart_instance_generation, 43);
        assert_eq!(
            terminal.charts[&7]
                .candle_fetch_request
                .as_ref()
                .map(|request| request.chart_instance_generation),
            Some(43)
        );
        assert_eq!(terminal.charts[&7].macro_candles_request_id, 1);
        assert_eq!(
            terminal.spaghetti_charts[&9].pending_spaghetti_candle_request_id("BTC"),
            Some(1)
        );
    }

    #[test]
    fn runtime_layout_restore_defers_unresolved_legacy_spot_aliases() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols.clear();
        let mut config = config::ChartConfig::empty(7, "@0", "H1");
        config.secondary_symbol = Some("@0".to_string());

        let _tasks = terminal.restore_layout_chart_instances(&[config], &[], 8, 0);

        let chart = &terminal.charts[&7];
        assert_eq!(chart.symbol, "@0");
        assert_eq!(chart.secondary_symbol.as_deref(), Some("@0"));
        assert!(chart.candle_fetch_request.is_none());
        assert!(chart.secondary_candle_fetch_request.is_none());
        assert_eq!(chart.macro_candles_request_id, 0);
    }
}
