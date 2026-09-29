use super::*;
use crate::api::Candle;
use crate::app_state::sensitive_string;
use crate::chart_state::ChartInstance;
use crate::hyperdash_api::{HeatmapFetchParams, HeatmapRect, LiquidationHeatmap};
use crate::timeframe::Timeframe;

#[test]
fn heatmap_cache_hits_preserve_order_waiters_and_independent_chart_cells() {
    for empty in [false, true] {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.hyperdash_api_key = sensitive_string("fixture-key");
        let end = (TradingTerminal::now_ms() / 3_600_000 - 1) * 3_600_000;
        let start = end - 3_600_000;
        let request = HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 9.5,
            max_price: 20.5,
            start_time: start / 1000,
            end_time: end / 1000,
        };
        let cache_key = request.cache_key();
        let data = LiquidationHeatmap {
            rects: if empty {
                Vec::new()
            } else {
                vec![HeatmapRect {
                    timestamp_ms: start,
                    duration_ms: 3_600_000,
                    price_lo: 10.0,
                    price_hi: 20.0,
                    amount_coins: -2.0,
                    amount_usd: -30.0,
                }]
            },
            max_abs_usd: if empty { 0.0 } else { 30.0 },
        };
        terminal.cache_heatmap_data(cache_key.clone(), data.clone());
        terminal.cache_heatmap_data("newer entry".to_string(), data.clone());
        let order = terminal.heatmap_data_cache_order.clone();
        terminal
            .heatmap_pending_charts
            .insert(cache_key.clone(), vec![42]);

        for id in [1, 2] {
            let mut instance = ChartInstance::new(id, "BTC".to_string(), Timeframe::H1);
            instance.show_heatmap = true;
            instance.chart.candles = vec![
                Candle::test_ohlcv(start, start + 3_599_999, [10.0, 20.0, 10.0, 15.0], 1.0),
                Candle::test_ohlcv(end, end + 3_599_999, [10.0, 20.0, 10.0, 15.0], 1.0),
            ];
            terminal.charts.insert(id, instance);

            let task = terminal.maybe_fetch_heatmap(id);

            assert_eq!(task.units(), 0);
            let instance = &terminal.charts[&id];
            assert_eq!(instance.heatmap_last_fetch.as_ref(), Some(&request));
            assert!(!instance.heatmap_fetching);
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
                        "HEAT hourly cached, 1 cells"
                    }
                    .to_string(),
                    empty
                ))
            );
        }

        assert_eq!(terminal.heatmap_data_cache_order, order);
        assert_eq!(
            terminal.heatmap_pending_charts.get(&cache_key),
            Some(&vec![42])
        );
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
            format!("{:?}", terminal.heatmap_data_cache[&cache_key]),
            format!("{data:?}")
        );
        assert!(terminal.toasts.is_empty());
    }
}
