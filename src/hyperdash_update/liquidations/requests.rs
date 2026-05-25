use super::planning::liquidation_request_key;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::hyperdash_api::fetch_liquidation_levels_at;
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// Liquidation Fetch Requests
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn queue_liquidation_fetch(
        &mut self,
        id: ChartId,
        coin: String,
        mark: f64,
    ) -> Task<Message> {
        self.queue_liquidation_fetch_at(id, coin, mark, Self::now_ms() / 1_000)
    }

    pub(super) fn queue_liquidation_fetch_at(
        &mut self,
        id: ChartId,
        coin: String,
        mark: f64,
        timestamp_secs: u64,
    ) -> Task<Message> {
        if self.hyperdash_api_key.is_empty() {
            return Task::none();
        }
        let min = 0.0;
        let max = mark * 2.0;
        let request_key = liquidation_request_key(&coin, min, max, timestamp_secs);

        if let Some(waiting_charts) = self.liquidation_pending_charts.get_mut(&request_key) {
            if !waiting_charts.contains(&id) {
                waiting_charts.push(id);
            }
            if let Some(instance) = self.charts.get_mut(&id) {
                instance.liquidation_fetching = true;
                instance.liquidation_pending_key = Some(request_key);
                instance.liquidation_status =
                    Some(("LIQ waiting for shared request".to_string(), false));
                instance.chart.candle_cache.clear();
            }
            return Task::none();
        }

        self.liquidation_pending_charts
            .insert(request_key.clone(), vec![id]);
        if let Some(instance) = self.charts.get_mut(&id) {
            instance.liquidation_fetching = true;
            instance.liquidation_pending_key = Some(request_key);
            instance.liquidation_status = Some(("LIQ loading".to_string(), false));
            instance.chart.candle_cache.clear();
        }

        let api_key = self.hyperdash_api_key.trim().to_string();
        let response_key = liquidation_request_key(&coin, min, max, timestamp_secs);
        Task::perform(
            fetch_liquidation_levels_at(coin, min, max, timestamp_secs, api_key),
            move |result| Message::ChartLiquidationLoaded(response_key.clone(), Box::new(result)),
        )
    }
}
