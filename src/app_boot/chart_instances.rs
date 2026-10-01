use crate::app_state::TradingTerminal;
use crate::chart_state::{CandleCacheTarget, ChartBackfillFetchContext, ChartId, ChartInstance};
use crate::config::{ChartBackfillSource, ChartConfig, SpaghettiChartConfig};
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::{Task, Theme};
use std::collections::{HashMap, HashSet};
use zeroize::Zeroizing;

impl TradingTerminal {
    pub(crate) fn boot_chart_instances(
        chart_configs: &[ChartConfig],
        muted_tickers: &HashSet<String>,
        chart_backfill_source: ChartBackfillSource,
        hydromancer_api_key: &Zeroizing<String>,
    ) -> (HashMap<ChartId, ChartInstance>, Vec<Task<Message>>) {
        let mut boot_tasks = Vec::new();
        let mut charts = HashMap::new();
        for chart_cfg in chart_configs {
            let id = chart_cfg.id;
            // `@0` is the legacy persisted key for the API-named PURR/USDC
            // pair. The candle endpoint rejects it, so wait for strict spot
            // metadata to supply the canonical key before loading cache or
            // issuing primary/macro requests.
            let defer_primary_legacy_spot = chart_cfg.symbol == "@0";
            let mut instance = ChartInstance::from_config(chart_cfg, chart_cfg.symbol.clone());
            let tf = instance.interval;
            if let Some(symbol) = chart_cfg.secondary_symbol.as_ref().filter(|symbol| {
                !symbol.is_empty() && !Self::key_matches_muted_tickers(&[], muted_tickers, symbol)
            }) {
                let display = symbol.split(':').nth(1).unwrap_or(symbol).to_string();
                instance.set_secondary_symbol_identity(symbol.clone(), display);
            }

            if !chart_cfg.symbol.is_empty()
                && !Self::key_matches_muted_tickers(&[], muted_tickers, &chart_cfg.symbol)
            {
                if tf.uses_candle_backfill() && !defer_primary_legacy_spot {
                    let source = if tf.requires_hydromancer_backfill() {
                        ChartBackfillSource::Hydromancer
                    } else {
                        chart_backfill_source
                    };
                    let can_load_cached_candles =
                        crate::api_cache::cache_eligible(source, tf, hydromancer_api_key);
                    let request = Self::build_candle_fetch_request(
                        id,
                        &chart_cfg.symbol,
                        tf,
                        crate::chart_state::ChartBackfillRequestContext::new(source, 0, 0),
                        None,
                        0,
                    )
                    .with_moving_average_history(&instance.macro_indicators);
                    instance.candle_fetch_request = Some(request.clone());
                    if can_load_cached_candles {
                        boot_tasks.push(Self::load_cached_candles_task(
                            request.clone(),
                            CandleCacheTarget::Primary,
                        ));
                    }
                    boot_tasks.push(Self::fetch_candles_task(
                        request,
                        hydromancer_api_key.clone(),
                    ));
                } else if !tf.uses_candle_backfill() {
                    instance.chart.status = crate::chart::ChartStatus::Loaded;
                }
                if !defer_primary_legacy_spot {
                    let macro_request_id = instance.next_macro_candles_request_id();
                    boot_tasks.extend(Self::fetch_macro_candles_tasks(
                        id,
                        macro_request_id,
                        &chart_cfg.symbol,
                        &instance.macro_indicators,
                    ));
                }
            } else if !chart_cfg.symbol.is_empty() {
                Self::clear_chart_for_muted_symbol(&mut instance);
            }
            if let Some(symbol) = instance.secondary_symbol.clone()
                && tf.uses_candle_backfill()
                && symbol != "@0"
            {
                let source = if tf.requires_hydromancer_backfill() {
                    ChartBackfillSource::Hydromancer
                } else {
                    chart_backfill_source
                };
                let can_load_cached_candles =
                    crate::api_cache::cache_eligible(source, tf, hydromancer_api_key);
                let request = Self::build_candle_fetch_request(
                    id,
                    &symbol,
                    tf,
                    crate::chart_state::ChartBackfillRequestContext::new(source, 0, 0),
                    None,
                    0,
                );
                instance.secondary_candle_fetch_request = Some(request.clone());
                if can_load_cached_candles {
                    boot_tasks.push(Self::load_cached_candles_task(
                        request.clone(),
                        CandleCacheTarget::Secondary,
                    ));
                }
                boot_tasks.push(Self::fetch_secondary_candles_task(
                    request,
                    hydromancer_api_key.clone(),
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

        for scfg in spaghetti_configs {
            let sid = scfg.id;
            let mut inst = SpaghettiChartInstance::from_config(scfg);
            let tf = inst.interval;
            Self::normalize_spaghetti_session_granularity(&mut inst, Self::now_ms());

            for sym_key in scfg
                .symbols
                .iter()
                .filter(|sym_key| !Self::key_matches_muted_tickers(&[], muted_tickers, sym_key))
            {
                let defer_legacy_api_named_pair = sym_key == "@0";
                let color_idx = inst.next_color_idx;
                inst.next_color_idx += 1;
                let colors = spaghetti::series_colors(&Theme::Dark);
                let color = colors[color_idx % colors.len()];
                let display = sym_key.split(':').nth(1).unwrap_or(sym_key).to_string();
                inst.canvas.series.push(spaghetti::Series {
                    symbol: sym_key.clone(),
                    display,
                    loaded: false,
                    candles: Vec::new(),
                    color,
                });
                if !defer_legacy_api_named_pair {
                    boot_tasks.push(Self::fetch_spaghetti_candles(
                        sid,
                        0,
                        sym_key,
                        tf,
                        inst.canvas.active_session,
                        inst.session_granularity,
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
