use super::*;

use crate::chart_state::ChartInstance;
use crate::hyperdash_api::{HeatmapFetchParams, HeatmapRect, LiquidationHeatmap};
use crate::timeframe::Timeframe;

#[test]
fn heatmap_success_keeps_cache_admission_and_waiter_gates() {
    for empty in [false, true] {
        for replacement in [false, true] {
            let (mut terminal, _) = TradingTerminal::boot();
            terminal.charts.clear();
            terminal.muted_tickers.insert("ETH".to_string());
            let request = HeatmapFetchParams {
                coin: "BTC".to_string(),
                min_price: 10.0,
                max_price: 20.0,
                start_time: 100,
                end_time: 200,
            };
            let cache_key = request.cache_key();
            let previous = LiquidationHeatmap {
                rects: Vec::new(),
                max_abs_usd: 9.0,
            };
            let data = LiquidationHeatmap {
                rects: if empty {
                    Vec::new()
                } else {
                    vec![HeatmapRect {
                        timestamp_ms: 100_000,
                        duration_ms: 3_600_000,
                        price_lo: 10.0,
                        price_hi: 20.0,
                        amount_coins: -2.0,
                        amount_usd: -30.0,
                    }]
                },
                max_abs_usd: if empty { 0.0 } else { 30.0 },
            };
            for index in 0..8 {
                let key = if replacement && index == 0 {
                    cache_key.clone()
                } else {
                    format!("old-{index}")
                };
                terminal.cache_heatmap_data(key, previous.clone());
            }
            for id in 1..=5 {
                let mut instance = ChartInstance::new(
                    id,
                    if id == 5 { "ETH" } else { "BTC" }.to_string(),
                    Timeframe::H1,
                );
                instance.show_heatmap = id != 3;
                instance.heatmap_fetching = true;
                let mut chart_request = request.clone();
                if id == 4 {
                    chart_request.start_time += 1;
                }
                instance.heatmap_last_fetch = Some(chart_request);
                instance.heatmap_data = Some(previous.clone());
                instance.chart.heatmap_max_usd = 9.0;
                instance.heatmap_status = Some(("previous status".to_string(), false));
                terminal.charts.insert(id, instance);
            }
            terminal
                .heatmap_pending_charts
                .insert(cache_key.clone(), vec![1, 2, 3, 4, 5, 6]);

            terminal.apply_chart_heatmap_loaded(
                cache_key.clone(),
                terminal.hyperdash_key_generation,
                Ok(data.clone()),
            );

            assert!(terminal.heatmap_pending_charts.is_empty());
            assert_eq!(terminal.heatmap_data_cache.len(), 8);
            let expected_order: Vec<_> = (1..8)
                .map(|index| format!("old-{index}"))
                .chain([cache_key.clone()])
                .collect();
            assert_eq!(
                terminal
                    .heatmap_data_cache_order
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
                expected_order
            );
            assert_eq!(
                format!("{:?}", terminal.heatmap_data_cache[&cache_key]),
                format!("{data:?}")
            );
            for id in 1..=5 {
                let instance = &terminal.charts[&id];
                assert_eq!(instance.heatmap_fetching, id == 5);
                if id <= 2 {
                    assert_eq!(
                        format!("{:?}", instance.chart.heatmap_rects),
                        format!("{:?}", data.rects)
                    );
                    assert_eq!(instance.chart.heatmap_max_usd, data.max_abs_usd);
                    let marker = instance.heatmap_data.as_ref().expect("loaded marker");
                    assert!(marker.rects.is_empty());
                    assert_eq!(marker.max_abs_usd, data.max_abs_usd);
                    assert_eq!(
                        instance.heatmap_status,
                        Some((
                            if empty {
                                "HEAT no recent data"
                            } else {
                                "HEAT hourly, 1 cells"
                            }
                            .to_string(),
                            empty
                        ))
                    );
                } else {
                    assert_eq!(instance.chart.heatmap_max_usd, 9.0);
                    assert_eq!(
                        instance.heatmap_status,
                        Some(("previous status".to_string(), false))
                    );
                    assert_eq!(
                        instance
                            .heatmap_data
                            .as_ref()
                            .expect("retained marker")
                            .max_abs_usd,
                        9.0
                    );
                }
            }
            terminal
                .charts
                .get_mut(&1)
                .expect("first chart")
                .chart
                .heatmap_rects
                .clear();
            assert_eq!(
                terminal.charts[&2].chart.heatmap_rects.len(),
                data.rects.len()
            );
            assert_eq!(
                terminal.heatmap_data_cache[&cache_key].rects.len(),
                data.rects.len()
            );
            assert!(terminal.toasts.is_empty());
        }
    }
}

#[test]
fn stale_hyperdash_generation_heatmap_result_keeps_current_pending_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
    terminal.hyperdash_key_generation = 2;
    terminal
        .heatmap_pending_charts
        .insert(cache_key.clone(), vec![7]);

    terminal.apply_chart_heatmap_loaded(
        cache_key.clone(),
        1,
        Ok(LiquidationHeatmap {
            rects: Vec::new(),
            max_abs_usd: 0.0,
        }),
    );

    assert_eq!(
        terminal.heatmap_pending_charts.get(&cache_key),
        Some(&vec![7])
    );
}

#[test]
fn heatmap_result_without_pending_charts_is_ignored() {
    let (mut terminal, _) = TradingTerminal::boot();
    let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
    let generation = terminal.hyperdash_key_generation;

    terminal.apply_chart_heatmap_loaded(
        cache_key.clone(),
        generation,
        Ok(LiquidationHeatmap {
            rects: Vec::new(),
            max_abs_usd: 0.0,
        }),
    );
    terminal.apply_chart_heatmap_loaded(
        cache_key.clone(),
        generation,
        Err("late failure".to_string()),
    );

    assert!(!terminal.heatmap_data_cache.contains_key(&cache_key));
    assert!(terminal.toasts.is_empty());
}

#[test]
fn disabling_heatmap_overlay_removes_pending_waiter_and_ignores_late_error() {
    let (mut terminal, _) = TradingTerminal::boot();
    let chart_id = 1;
    let cache_key = "BTC:1.00000000:2.00000000:10:20".to_string();
    let generation = terminal.hyperdash_key_generation;
    terminal.charts.clear();
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.show_heatmap = true;
    instance.heatmap_fetching = true;
    instance.heatmap_status = Some(("HEAT refreshing hourly data".to_string(), false));
    terminal.charts.insert(chart_id, instance);
    terminal
        .heatmap_pending_charts
        .insert(cache_key.clone(), vec![chart_id]);

    let _task = terminal.toggle_heatmap_overlay(chart_id);
    terminal.apply_chart_heatmap_loaded(
        cache_key.clone(),
        generation,
        Err("late failure".to_string()),
    );

    assert!(!terminal.heatmap_pending_charts.contains_key(&cache_key));
    assert!(terminal.toasts.is_empty());
    let instance = terminal.charts.get(&chart_id).expect("chart");
    assert!(!instance.show_heatmap);
    assert!(!instance.heatmap_fetching);
    assert!(instance.heatmap_status.is_none());
}

#[test]
fn late_heatmap_error_for_old_request_does_not_clear_current_request() {
    let (mut terminal, _) = TradingTerminal::boot();
    let chart_id = 1;
    let stale_request = HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 1.0,
        max_price: 2.0,
        start_time: 10,
        end_time: 20,
    };
    let current_request = HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 3.0,
        max_price: 4.0,
        start_time: 30,
        end_time: 40,
    };
    let stale_key = stale_request.cache_key();
    let current_key = current_request.cache_key();
    let generation = terminal.hyperdash_key_generation;
    terminal.charts.clear();
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.show_heatmap = true;
    instance.heatmap_fetching = true;
    instance.heatmap_last_fetch = Some(current_request);
    instance.heatmap_status = Some(("HEAT refreshing current data".to_string(), false));
    terminal.charts.insert(chart_id, instance);
    terminal
        .heatmap_pending_charts
        .insert(stale_key.clone(), vec![chart_id]);

    terminal.apply_chart_heatmap_loaded(stale_key, generation, Err("late failure".to_string()));

    assert!(terminal.toasts.is_empty());
    let instance = terminal.charts.get(&chart_id).expect("chart");
    assert!(instance.heatmap_fetching);
    assert_eq!(
        instance
            .heatmap_last_fetch
            .as_ref()
            .map(HeatmapFetchParams::cache_key)
            .as_deref(),
        Some(current_key.as_str())
    );
    assert_eq!(
        instance
            .heatmap_status
            .as_ref()
            .map(|(message, is_error)| { (message.as_str(), *is_error) }),
        Some(("HEAT refreshing current data", false))
    );
}

#[test]
fn current_heatmap_error_redacts_toast_detail() {
    let (mut terminal, _) = TradingTerminal::boot();
    let chart_id = 1;
    let request = HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 1.0,
        max_price: 2.0,
        start_time: 10,
        end_time: 20,
    };
    let cache_key = request.cache_key();
    let generation = terminal.hyperdash_key_generation;
    terminal.charts.clear();
    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.show_heatmap = true;
    instance.heatmap_fetching = true;
    instance.heatmap_last_fetch = Some(request);
    instance.heatmap_status = Some(("HEAT refreshing current data".to_string(), false));
    terminal.charts.insert(chart_id, instance);
    terminal
        .heatmap_pending_charts
        .insert(cache_key.clone(), vec![chart_id]);

    terminal.apply_chart_heatmap_loaded(
        cache_key,
        generation,
        Err("heatmap rejected: api_key=key-secret signature=sig-secret".to_string()),
    );

    let instance = terminal.charts.get(&chart_id).expect("chart");
    assert!(!instance.heatmap_fetching);
    assert!(instance.heatmap_last_fetch.is_none());
    assert_eq!(
        instance
            .heatmap_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("HEAT fetch failed", true))
    );

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("api_key=<redacted>"));
    assert!(toast.message.contains("signature=<redacted>"));
    assert!(!toast.message.contains("key-secret"));
    assert!(!toast.message.contains("sig-secret"));
}
