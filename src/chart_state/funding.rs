use super::{ChartId, ChartInstance, FundingFetchMode, FundingFetchRequest};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::hydromancer_api::{FundingRatePoint, fetch_funding_history};
use crate::message::{Message, RedactedPublicMarketMessageResult};
use iced::Task;

mod planning;

use self::planning::{FundingRequestPlan, plan_funding_request};

// ---------------------------------------------------------------------------
// Funding History Fetching
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn maybe_fetch_chart_funding(&mut self, chart_id: ChartId) -> Task<Message> {
        let api_key = self.hydromancer_api_key_for_task();
        let hydromancer_key_generation = self.hydromancer_key_generation;
        let now_ms = Self::now_ms();
        let planned = {
            let Some(instance) = self.charts.get(&chart_id) else {
                return Task::none();
            };
            if !instance.macro_indicators.show_funding_rate || instance.symbol.is_empty() {
                return Task::none();
            }
            if api_key.is_empty() {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.funding_fetch_request = None;
                    instance.funding_last_attempt_ms = None;
                    instance.chart.funding_rates.clear();
                    instance.chart.set_funding_status(
                        "Add Hydromancer key in Settings > Integrations".to_string(),
                        true,
                    );
                }
                return Task::none();
            }
            if instance.funding_fetch_request.is_some() {
                return Task::none();
            }

            let status = plan_funding_request(
                instance,
                self.symbol_key_is_hidden(&instance.symbol),
                self.hyperdash_coin_for_symbol(&instance.symbol),
                now_ms,
                self.chart_instance_generation,
                instance.funding_request_id.wrapping_add(1),
                hydromancer_key_generation,
                false,
            );
            match status {
                FundingRequestPlan::Fetch(request) => request,
                FundingRequestPlan::Wait => return Task::none(),
                FundingRequestPlan::Status(label, is_error) => {
                    if let Some(instance) = self.charts.get_mut(&chart_id) {
                        instance.funding_fetch_request = None;
                        instance.funding_last_attempt_ms = None;
                        instance.chart.funding_rates.clear();
                        instance.chart.set_funding_status(label, is_error);
                    }
                    return Task::none();
                }
            }
        };

        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.funding_request_id = planned.request_id;
            instance.funding_fetch_request = Some(planned.clone());
            instance.funding_last_attempt_ms = Some(now_ms);
            instance
                .chart
                .set_funding_status("Funding loading".to_string(), false);
        }

        Task::perform(
            fetch_funding_history(
                planned.coin.clone(),
                planned.start_ms,
                planned.end_ms,
                api_key,
            ),
            move |result| Message::ChartFundingHistoryLoaded(planned.clone(), result.into()),
        )
    }

    pub(crate) fn refresh_due_funding_charts(&mut self) -> Task<Message> {
        let ids: Vec<ChartId> = self
            .charts
            .iter()
            .filter(|(_, instance)| instance.macro_indicators.show_funding_rate)
            .map(|(id, _)| *id)
            .collect();

        Task::batch(
            ids.into_iter()
                .map(|chart_id| self.maybe_fetch_chart_funding(chart_id)),
        )
    }

    pub(crate) fn refresh_enabled_funding_charts(&mut self) -> Task<Message> {
        let ids: Vec<ChartId> = self
            .charts
            .iter()
            .filter(|(_, instance)| instance.macro_indicators.show_funding_rate)
            .map(|(id, _)| *id)
            .collect();
        if ids.is_empty() {
            return Task::none();
        }

        for chart_id in &ids {
            if let Some(instance) = self.charts.get_mut(chart_id) {
                instance.funding_fetch_request = None;
                instance.funding_last_attempt_ms = None;
            }
        }

        Task::batch(
            ids.into_iter()
                .map(|chart_id| self.maybe_fetch_chart_funding(chart_id)),
        )
    }

    pub(crate) fn clear_funding_display(instance: &mut ChartInstance) {
        instance.funding_fetch_request = None;
        instance.funding_last_attempt_ms = None;
        instance.chart.clear_funding_history();
    }

    pub(crate) fn apply_chart_funding_history_loaded(
        &mut self,
        request: FundingFetchRequest,
        result: RedactedPublicMarketMessageResult<Vec<FundingRatePoint>>,
    ) -> Task<Message> {
        if request.chart_instance_generation != self.chart_instance_generation {
            return Task::none();
        }
        if !self.hydromancer_key_generation_is_current(request.hydromancer_key_generation) {
            return Task::none();
        }

        if self
            .charts
            .get(&request.chart_id)
            .is_some_and(|instance| self.symbol_key_is_hidden(&instance.symbol))
        {
            return Task::none();
        }

        let mut toast = None;
        if let Some(instance) = self.charts.get_mut(&request.chart_id) {
            let request_matches = instance.funding_fetch_request.as_ref() == Some(&request)
                && instance.symbol == request.symbol
                && instance.macro_indicators.show_funding_rate;
            if !request_matches {
                if instance.funding_fetch_request.as_ref() == Some(&request) {
                    instance.funding_fetch_request = None;
                }
                return Task::none();
            }

            instance.funding_fetch_request = None;
            match result.into_result() {
                Ok(points) => match request.mode {
                    FundingFetchMode::Snapshot => instance.chart.set_funding_history(points),
                    FundingFetchMode::Incremental => {
                        instance.chart.merge_funding_history(points);
                    }
                },
                Err(error) => {
                    instance
                        .chart
                        .set_funding_status("Funding fetch failed".to_string(), true);
                    toast = Some(format!(
                        "Funding fetch failed: {}",
                        redact_sensitive_response_text(&error)
                    ));
                }
            }
        }

        if let Some(message) = toast {
            self.push_toast(message, true);
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests;
