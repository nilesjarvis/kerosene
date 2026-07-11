use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartBackfillFetchContext, ChartId, ChartInstance};
use crate::config::{ChartBackfillSource, ChartConfig, SpaghettiChartConfig};
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::Timeframe;
use iced::{Task, Theme};
use std::collections::{HashMap, HashSet};
use zeroize::Zeroizing;

impl TradingTerminal {
    pub(crate) fn boot_chart_instances(
        chart_configs: &[ChartConfig],
        muted_tickers: &HashSet<String>,
        chart_backfill_source: ChartBackfillSource,
        hydromancer_api_key: &Zeroizing<String>,
        schwab_access_token: &Zeroizing<String>,
    ) -> (HashMap<ChartId, ChartInstance>, Vec<Task<Message>>) {
        let mut boot_tasks = Vec::new();
        let mut charts = HashMap::new();
        let now_ms = Self::now_ms();

        for chart_cfg in chart_configs {
            let id = chart_cfg.id;
            let tf = Timeframe::from_config_str(&chart_cfg.timeframe);
            // `@0` is the legacy persisted key for the API-named PURR/USDC
            // pair. The candle endpoint rejects it, so wait for strict spot
            // metadata to supply the canonical key before loading cache or
            // issuing primary/macro requests.
            let defer_primary_legacy_spot = chart_cfg.symbol == "@0";
            let mut instance = ChartInstance::new(id, chart_cfg.symbol.clone(), tf);
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
            if let Some(symbol) = chart_cfg.secondary_symbol.as_ref().filter(|symbol| {
                !symbol.is_empty() && !Self::key_matches_muted_tickers(&[], muted_tickers, symbol)
            }) {
                let display = symbol.split(':').nth(1).unwrap_or(symbol).to_string();
                instance.set_secondary_symbol_identity(symbol.clone(), display);
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

            if !chart_cfg.symbol.is_empty()
                && !Self::key_matches_muted_tickers(&[], muted_tickers, &chart_cfg.symbol)
            {
                if tf.uses_candle_backfill() && !defer_primary_legacy_spot {
                    let source = if crate::schwab::is_schwab_symbol_key(&chart_cfg.symbol) {
                        ChartBackfillSource::Schwab
                    } else if tf.requires_hydromancer_backfill() {
                        ChartBackfillSource::Hydromancer
                    } else {
                        chart_backfill_source
                    };
                    let can_load_cached_candles = (source != ChartBackfillSource::Schwab
                        || !schwab_access_token.trim().is_empty())
                        && crate::api_cache::cache_eligible(source, tf, hydromancer_api_key);
                    let cached_candles = can_load_cached_candles
                        .then(|| {
                            crate::api_cache::load_fresh_candles(
                                source,
                                &chart_cfg.symbol,
                                tf,
                                now_ms,
                            )
                            .ok()
                            .flatten()
                        })
                        .flatten();
                    let cached_start_ms = cached_candles
                        .as_ref()
                        .and_then(|candles| candles.last().map(|candle| candle.open_time));
                    if let Some(candles) = cached_candles {
                        instance.chart.set_candles(candles);
                    }
                    let request = Self::build_candle_fetch_request(
                        id,
                        &chart_cfg.symbol,
                        tf,
                        crate::chart_state::ChartBackfillRequestContext::new(source, 0, 0, 0),
                        cached_start_ms,
                        0,
                    );
                    instance.candle_fetch_request = Some(request.clone());
                    boot_tasks.push(Self::fetch_candles_task(
                        request,
                        hydromancer_api_key.clone(),
                        schwab_access_token.clone(),
                    ));
                } else if !tf.uses_candle_backfill() {
                    instance.chart.status = crate::chart::ChartStatus::Loaded;
                }
                if !defer_primary_legacy_spot {
                    let macro_request_id = instance.next_macro_candles_request_id();
                    boot_tasks.extend(Self::fetch_macro_candles_tasks(
                        id,
                        0,
                        macro_request_id,
                        &chart_cfg.symbol,
                    ));
                }
            } else if !chart_cfg.symbol.is_empty() {
                Self::clear_chart_for_muted_symbol(&mut instance);
            }
            if let Some(symbol) = instance.secondary_symbol.clone()
                && tf.uses_candle_backfill()
                && symbol != "@0"
            {
                let source = if crate::schwab::is_schwab_symbol_key(&symbol) {
                    ChartBackfillSource::Schwab
                } else if tf.requires_hydromancer_backfill() {
                    ChartBackfillSource::Hydromancer
                } else {
                    chart_backfill_source
                };
                let can_load_cached_candles = (source != ChartBackfillSource::Schwab
                    || !schwab_access_token.trim().is_empty())
                    && crate::api_cache::cache_eligible(source, tf, hydromancer_api_key);
                let cached_candles = can_load_cached_candles
                    .then(|| {
                        crate::api_cache::load_fresh_candles(source, &symbol, tf, now_ms)
                            .ok()
                            .flatten()
                    })
                    .flatten();
                let cached_start_ms = cached_candles
                    .as_ref()
                    .and_then(|candles| candles.last().map(|candle| candle.open_time));
                if let Some(candles) = cached_candles {
                    instance.chart.set_secondary_candles(candles);
                }
                let request = Self::build_candle_fetch_request(
                    id,
                    &symbol,
                    tf,
                    crate::chart_state::ChartBackfillRequestContext::new(source, 0, 0, 0),
                    cached_start_ms,
                    0,
                );
                instance.secondary_candle_fetch_request = Some(request.clone());
                boot_tasks.push(Self::fetch_secondary_candles_task(
                    request,
                    hydromancer_api_key.clone(),
                    schwab_access_token.clone(),
                ));
            }

            charts.insert(id, instance);
        }

        (charts, boot_tasks)
    }

    pub(crate) fn boot_spaghetti_instances(
        spaghetti_configs: &[SpaghettiChartConfig],
        muted_tickers: &HashSet<String>,
        chart_backfill_source: ChartBackfillSource,
        hydromancer_api_key: &Zeroizing<String>,
    ) -> (
        HashMap<SpaghettiChartId, SpaghettiChartInstance>,
        Vec<Task<Message>>,
    ) {
        let mut boot_tasks = Vec::new();
        let mut spaghetti_charts = HashMap::new();
        let now_ms = Self::now_ms();

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
                let defer_legacy_api_named_pair = sym_key == "@0";
                let effective_tf = Self::spaghetti_effective_timeframe_for(
                    tf,
                    inst.canvas.active_session,
                    inst.session_granularity,
                    now_ms,
                );
                let can_load_cached_candles = !defer_legacy_api_named_pair
                    && crate::api_cache::cache_eligible(
                        chart_backfill_source,
                        effective_tf,
                        hydromancer_api_key,
                    );
                let cached_candles = can_load_cached_candles
                    .then(|| {
                        crate::api_cache::load_fresh_candles(
                            chart_backfill_source,
                            sym_key,
                            effective_tf,
                            now_ms,
                        )
                        .ok()
                        .flatten()
                    })
                    .flatten();
                let cached_start_ms = cached_candles
                    .as_ref()
                    .and_then(|candles| candles.last().map(|candle| candle.open_time));
                let color_idx = inst.next_color_idx;
                inst.next_color_idx += 1;
                let colors = spaghetti::series_colors(&Theme::Dark);
                let color = colors[color_idx % colors.len()];
                let display = sym_key.split(':').nth(1).unwrap_or(sym_key).to_string();
                inst.canvas.series.push(spaghetti::Series {
                    symbol: sym_key.clone(),
                    display,
                    loaded: cached_candles.is_some(),
                    candles: cached_candles.unwrap_or_default(),
                    color,
                });
                if !defer_legacy_api_named_pair {
                    boot_tasks.push(Self::queue_spaghetti_candle_fetch(
                        &mut inst,
                        sym_key,
                        0,
                        cached_start_ms,
                        ChartBackfillFetchContext::new(
                            chart_backfill_source,
                            0,
                            0,
                            hydromancer_api_key.clone(),
                        ),
                    ));
                }
            }

            spaghetti_charts.insert(sid, inst);
        }

        (spaghetti_charts, boot_tasks)
    }
}

#[cfg(test)]
mod tests;
