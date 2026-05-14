use super::{ChartId, ChartInstance, FundingFetchMode, FundingFetchRequest};
use crate::app_state::TradingTerminal;
use crate::hydromancer_api::{FundingRatePoint, fetch_funding_history};
use crate::message::Message;
use iced::Task;

// ---------------------------------------------------------------------------
// Funding History Fetching
// ---------------------------------------------------------------------------

const FUNDING_HISTORY_INCREMENT_MS: u64 = 60 * 60 * 1_000;
const FUNDING_INCREMENTAL_RETRY_MS: u64 = 5 * 60 * 1_000;
const FUNDING_EMPTY_SNAPSHOT_RETRY_MS: u64 = 15 * 60 * 1_000;

impl TradingTerminal {
    pub(crate) fn maybe_fetch_chart_funding(&mut self, chart_id: ChartId) -> Task<Message> {
        let api_key = self.hydromancer_api_key.trim().to_string();
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
            move |result| Message::ChartFundingHistoryLoaded(planned.clone(), Box::new(result)),
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
        result: Result<Vec<FundingRatePoint>, String>,
    ) -> Task<Message> {
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
            match result {
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
                    toast = Some(format!("Funding fetch failed: {error}"));
                }
            }
        }

        if let Some(message) = toast {
            self.push_toast(message, true);
        }

        Task::none()
    }
}

enum FundingRequestPlan {
    Fetch(FundingFetchRequest),
    Wait,
    Status(String, bool),
}

fn plan_funding_request(
    instance: &ChartInstance,
    muted: bool,
    coin: Option<String>,
    now_ms: u64,
    api_key_missing: bool,
) -> FundingRequestPlan {
    if muted {
        return FundingRequestPlan::Status("Ticker is hidden in Settings > Risk".to_string(), true);
    }
    if api_key_missing {
        return FundingRequestPlan::Status(
            "Add Hydromancer key in Settings > Integrations".to_string(),
            true,
        );
    }
    let Some(coin) = coin else {
        return FundingRequestPlan::Status("Funding requires a perp symbol".to_string(), true);
    };
    let Some((start_ms, end_ms)) = funding_time_range(
        &instance.chart.candles,
        instance.interval.duration_ms(),
        now_ms,
    ) else {
        return FundingRequestPlan::Wait;
    };

    if let Some(latest_time_ms) = instance
        .chart
        .funding_rates
        .iter()
        .map(|point| point.time_ms)
        .max()
    {
        if !funding_incremental_due(latest_time_ms, end_ms)
            || !funding_attempt_allowed(
                instance.funding_last_attempt_ms,
                now_ms,
                FUNDING_INCREMENTAL_RETRY_MS,
            )
        {
            return FundingRequestPlan::Wait;
        }

        return FundingRequestPlan::Fetch(FundingFetchRequest {
            chart_id: instance.id,
            symbol: instance.symbol.clone(),
            coin,
            start_ms: latest_time_ms,
            end_ms,
            mode: FundingFetchMode::Incremental,
        });
    }

    if !funding_attempt_allowed(
        instance.funding_last_attempt_ms,
        now_ms,
        FUNDING_EMPTY_SNAPSHOT_RETRY_MS,
    ) {
        return FundingRequestPlan::Wait;
    }

    FundingRequestPlan::Fetch(FundingFetchRequest {
        chart_id: instance.id,
        symbol: instance.symbol.clone(),
        coin,
        start_ms,
        end_ms,
        mode: FundingFetchMode::Snapshot,
    })
}

fn funding_time_range(
    candles: &[crate::api::Candle],
    interval_ms: u64,
    now_ms: u64,
) -> Option<(u64, u64)> {
    let first = candles.first()?.open_time;
    let last = candles.last()?.open_time;
    let end = last.saturating_add(interval_ms).min(now_ms.max(last));
    (end > first).then_some((first, end))
}

fn funding_incremental_due(latest_time_ms: u64, target_end_ms: u64) -> bool {
    target_end_ms >= latest_time_ms.saturating_add(FUNDING_HISTORY_INCREMENT_MS)
}

fn funding_attempt_allowed(
    last_attempt_ms: Option<u64>,
    now_ms: u64,
    min_interval_ms: u64,
) -> bool {
    match last_attempt_ms {
        Some(last) => now_ms.saturating_sub(last) >= min_interval_ms,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{funding_attempt_allowed, funding_incremental_due, funding_time_range};
    use crate::api::Candle;

    fn candle(open_time: u64) -> Candle {
        Candle {
            open_time,
            close_time: open_time,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        }
    }

    #[test]
    fn funding_range_uses_first_candle_and_caps_end_at_now() {
        let candles = [candle(1_000), candle(2_000)];

        assert_eq!(
            funding_time_range(&candles, 3_600_000, 3_000),
            Some((1_000, 3_000))
        );
    }

    #[test]
    fn funding_range_waits_without_candles_or_duration() {
        assert_eq!(funding_time_range(&[], 3_600_000, 3_000), None);
        assert_eq!(funding_time_range(&[candle(1_000)], 0, 1_000), None);
    }

    #[test]
    fn incremental_funding_waits_until_next_hourly_bucket() {
        assert!(!funding_incremental_due(1_000, 3_600_999));
        assert!(funding_incremental_due(1_000, 3_601_000));
    }

    #[test]
    fn funding_attempts_are_throttled() {
        assert!(funding_attempt_allowed(None, 10_000, 5_000));
        assert!(!funding_attempt_allowed(Some(8_000), 10_000, 5_000));
        assert!(funding_attempt_allowed(Some(5_000), 10_000, 5_000));
    }
}
