use super::LIQUIDATION_BUCKET_COUNT;
use super::display::reset_liquidation_fetch_state;
use super::planning::liquidation_request_coin;
use crate::app_state::TradingTerminal;
use crate::hyperdash_api::{LiquidationLevel, bucket_liquidations};
use crate::message::Message;
use iced::Task;

// ---------------------------------------------------------------------------
// Liquidation Result Application
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::hyperdash_update) fn apply_chart_liquidation_loaded(
        &mut self,
        request_key: String,
        result: Result<LiquidationLevel, String>,
    ) -> Task<Message> {
        let waiting_charts = self
            .liquidation_pending_charts
            .remove(&request_key)
            .unwrap_or_default();
        if waiting_charts.is_empty() {
            return Task::none();
        }

        let mut toast = None;
        match result {
            Ok(data) => {
                let buckets = bucket_liquidations(
                    &data.liquidations,
                    data.min,
                    data.max,
                    LIQUIDATION_BUCKET_COUNT,
                );
                for id in waiting_charts {
                    if !self.chart_can_accept_liquidation_result(id, &data.coin) {
                        if let Some(instance) = self.charts.get_mut(&id) {
                            reset_liquidation_fetch_state(instance);
                        }
                        continue;
                    }
                    if let Some(instance) = self.charts.get_mut(&id) {
                        instance.chart.liquidation_buckets = buckets.clone();
                        instance.liquidation_data = Some(data.clone());
                        reset_liquidation_fetch_state(instance);
                        instance.liquidation_status = Some(("LIQ loaded".to_string(), false));
                        instance.chart.candle_cache.clear();
                    }
                }
            }
            Err(error) => {
                let request_coin = liquidation_request_coin(&request_key);
                let mut failed_visible_chart = false;
                for id in waiting_charts {
                    let can_accept_failure =
                        self.chart_can_accept_liquidation_result(id, request_coin);
                    if let Some(instance) = self.charts.get_mut(&id) {
                        reset_liquidation_fetch_state(instance);
                        if !can_accept_failure {
                            continue;
                        }
                        instance.liquidation_status = Some(("LIQ fetch failed".to_string(), true));
                        if instance.liquidation_data.is_none() {
                            instance.chart.liquidation_buckets.clear();
                        }
                        instance.chart.candle_cache.clear();
                        failed_visible_chart = true;
                    }
                }
                if failed_visible_chart {
                    toast = Some(format!("LIQ fetch failed: {error}"));
                }
            }
        }

        if let Some(message) = toast {
            self.push_toast(message, true);
        }

        Task::none()
    }
}
