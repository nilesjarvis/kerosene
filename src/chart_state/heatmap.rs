mod cache;
mod range;
mod request;

use super::ChartId;
use crate::app_state::TradingTerminal;
use crate::hyperdash_api::fetch_liquidation_heatmap;
use crate::message::Message;
use iced::Task;

use self::request::{HeatmapRequestContext, plan_heatmap_fetch_request};

impl TradingTerminal {
    /// Fetch heatmap data for a chart if the overlay is enabled and we
    /// have candle data to derive the visible price/time range.
    pub(crate) fn maybe_fetch_heatmap(&mut self, chart_id: ChartId) -> Task<Message> {
        let _theme = self.theme();
        if self.hyperdash_api_key.is_empty() {
            return Task::none();
        }

        let planned_request = {
            let Some(instance) = self.charts.get(&chart_id) else {
                return Task::none();
            };
            plan_heatmap_fetch_request(HeatmapRequestContext {
                show_heatmap: instance.show_heatmap,
                symbol: &instance.symbol,
                heatmap_fetching: instance.heatmap_fetching,
                muted: self.symbol_key_is_hidden(&instance.symbol),
                coin: self.hyperdash_coin_for_symbol(&instance.symbol),
                candles: &instance.chart.candles,
                viewport: instance.heatmap_viewport,
                previous: instance.heatmap_last_fetch.as_ref(),
                now_time: Self::now_ms() / 1000,
            })
        };

        let request = match planned_request {
            Ok(Some(request)) => request,
            Ok(None) => return Task::none(),
            Err(status) => {
                if let Some(inst) = self.charts.get_mut(&chart_id) {
                    inst.heatmap_fetching = false;
                    inst.heatmap_status = Some((status, true));
                    Self::clear_heatmap_display(inst);
                }
                return Task::none();
            }
        };

        let cache_key = request.cache_key();
        if let Some(data) = self.heatmap_data_cache.get(&cache_key).cloned() {
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.heatmap_last_fetch = Some(request);
            }
            self.apply_heatmap_data_to_chart(chart_id, &cache_key, &data, true);
            return Task::none();
        }

        if let Some(waiting_charts) = self.heatmap_pending_charts.get_mut(&cache_key) {
            if !waiting_charts.contains(&chart_id) {
                waiting_charts.push(chart_id);
            }
            if let Some(inst) = self.charts.get_mut(&chart_id) {
                inst.heatmap_fetching = true;
                inst.heatmap_last_fetch = Some(request);
                inst.heatmap_status = Some(("HEAT waiting for shared request".to_string(), false));
            }
            return Task::none();
        }

        self.heatmap_pending_charts
            .insert(cache_key.clone(), vec![chart_id]);
        if let Some(inst) = self.charts.get_mut(&chart_id) {
            inst.heatmap_fetching = true;
            inst.heatmap_last_fetch = Some(request.clone());
            inst.heatmap_status = Some(("HEAT refreshing hourly data".to_string(), false));
        }

        let api_key = self.hyperdash_api_key.trim().to_string();
        let key = cache_key;
        Task::perform(
            fetch_liquidation_heatmap(
                request.coin,
                request.min_price,
                request.max_price,
                request.start_time,
                request.end_time,
                api_key,
            ),
            move |r| Message::ChartHeatmapLoaded(key, Box::new(r)),
        )
    }
}
