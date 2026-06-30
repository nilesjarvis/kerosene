use super::super::{ChartId, ChartInstance};
use crate::app_state::TradingTerminal;
use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap};

// ---------------------------------------------------------------------------
// Heatmap Cache And Display
// ---------------------------------------------------------------------------

const HEATMAP_CACHE_MAX_ENTRIES: usize = 8;

impl TradingTerminal {
    pub(crate) fn cache_heatmap_data(&mut self, cache_key: String, data: LiquidationHeatmap) {
        self.heatmap_data_cache_order.retain(|k| k != &cache_key);
        self.heatmap_data_cache.insert(cache_key.clone(), data);
        self.heatmap_data_cache_order.push_back(cache_key);

        while self.heatmap_data_cache_order.len() > HEATMAP_CACHE_MAX_ENTRIES {
            if let Some(oldest) = self.heatmap_data_cache_order.pop_front() {
                self.heatmap_data_cache.remove(&oldest);
            }
        }
    }

    pub(crate) fn clear_heatmap_display(instance: &mut ChartInstance) {
        instance.heatmap_data = None;
        instance.chart.heatmap_rects.clear();
        instance.chart.heatmap_max_usd = 0.0;
        instance.chart.candle_cache.clear();
    }

    pub(crate) fn apply_heatmap_data_to_chart(
        &mut self,
        chart_id: ChartId,
        cache_key: &str,
        data: &LiquidationHeatmap,
        from_cache: bool,
    ) {
        let muted = self
            .charts
            .get(&chart_id)
            .is_some_and(|instance| self.symbol_key_is_hidden(&instance.symbol));
        if muted {
            return;
        }
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            let requested_key = instance
                .heatmap_last_fetch
                .as_ref()
                .map(HeatmapFetchParams::cache_key);
            if !instance.show_heatmap || requested_key.as_deref() != Some(cache_key) {
                instance.heatmap_fetching = false;
                return;
            }

            instance.chart.heatmap_rects = data.rects.clone();
            instance.chart.heatmap_max_usd = data.max_abs_usd;
            instance.heatmap_data = Some(LiquidationHeatmap {
                rects: Vec::new(),
                max_abs_usd: data.max_abs_usd,
            });
            instance.heatmap_fetching = false;
            instance.heatmap_status = Some((
                if data.rects.is_empty() {
                    "HEAT no recent data".to_string()
                } else if from_cache {
                    format!("HEAT hourly cached, {} cells", data.rects.len())
                } else {
                    format!("HEAT hourly, {} cells", data.rects.len())
                },
                data.rects.is_empty(),
            ));
            instance.chart.candle_cache.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::hyperdash_api::HeatmapRect;
    use crate::timeframe::Timeframe;

    #[test]
    fn applied_heatmap_keeps_full_rects_only_on_chart() {
        let (mut terminal, _) = TradingTerminal::boot();
        let chart_id = 1;
        let request = HeatmapFetchParams {
            coin: "BTC".to_string(),
            min_price: 10.0,
            max_price: 20.0,
            start_time: 100,
            end_time: 200,
        };
        let cache_key = request.cache_key();
        let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
        instance.show_heatmap = true;
        instance.heatmap_last_fetch = Some(request);
        terminal.charts.insert(chart_id, instance);

        let data = LiquidationHeatmap {
            rects: vec![HeatmapRect {
                timestamp_ms: 100_000,
                duration_ms: 3_600_000,
                price_lo: 10.0,
                price_hi: 20.0,
                amount_coins: 1.0,
                amount_usd: 15.0,
            }],
            max_abs_usd: 15.0,
        };

        terminal.apply_heatmap_data_to_chart(chart_id, &cache_key, &data, false);

        let instance = terminal.charts.get(&chart_id).expect("chart should exist");
        assert_eq!(instance.chart.heatmap_rects.len(), 1);
        assert_eq!(
            instance
                .heatmap_data
                .as_ref()
                .expect("loaded marker should be set")
                .rects
                .len(),
            0
        );
        assert_eq!(instance.chart.heatmap_max_usd, 15.0);
    }
}
