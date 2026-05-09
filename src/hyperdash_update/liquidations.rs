use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::hyperdash_api::{LiquidationLevel, bucket_liquidations, fetch_liquidation_levels_at};
use crate::message::Message;
use iced::Task;

const LIQUIDATION_BUCKET_COUNT: usize = 200;

impl TradingTerminal {
    pub(in crate::hyperdash_update) fn toggle_liquidation_overlay(
        &mut self,
        chart_id: ChartId,
    ) -> Task<Message> {
        let Some(is_enabled) = self
            .charts
            .get(&chart_id)
            .map(|instance| instance.show_liquidations)
        else {
            return Task::none();
        };

        if is_enabled {
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.show_liquidations = false;
                Self::clear_liquidation_display(instance);
            }
            return Task::none();
        }

        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.show_liquidations = true;
        }
        let plan = self
            .charts
            .get(&chart_id)
            .map(|instance| self.plan_liquidation_fetch_for_instance(instance))
            .unwrap_or(LiquidationRequestPlan::Wait);

        match plan {
            LiquidationRequestPlan::Fetch { coin, mark } => {
                self.queue_liquidation_fetch(chart_id, coin, mark)
            }
            LiquidationRequestPlan::Status(message, is_error) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.liquidation_status = Some((message.clone(), is_error));
                    instance.chart.candle_cache.clear();
                }
                if is_error {
                    self.push_toast(message, true);
                }
                Task::none()
            }
            LiquidationRequestPlan::Wait => Task::none(),
        }
    }

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
                            instance.liquidation_fetching = false;
                            instance.liquidation_pending_key = None;
                        }
                        continue;
                    }
                    if let Some(instance) = self.charts.get_mut(&id) {
                        instance.chart.liquidation_buckets = buckets.clone();
                        instance.liquidation_data = Some(data.clone());
                        instance.liquidation_fetching = false;
                        instance.liquidation_pending_key = None;
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
                        instance.liquidation_fetching = false;
                        instance.liquidation_pending_key = None;
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

    pub(in crate::hyperdash_update) fn refresh_liquidations(&mut self) -> Task<Message> {
        if self.hyperdash_api_key.is_empty() {
            return Task::none();
        }
        let plans: Vec<(ChartId, String, f64)> = self
            .charts
            .values()
            .filter(|instance| instance.show_liquidations)
            .filter_map(|instance| {
                let LiquidationRequestPlan::Fetch { coin, mark } =
                    self.plan_liquidation_fetch_for_instance(instance)
                else {
                    return None;
                };
                Some((instance.id, coin, mark))
            })
            .collect();
        let now_secs = Self::now_ms() / 1_000;
        let tasks: Vec<Task<Message>> = plans
            .into_iter()
            .map(|(id, coin, mark)| self.queue_liquidation_fetch_at(id, coin, mark, now_secs))
            .collect();
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    pub(crate) fn maybe_fetch_liquidations(&mut self, chart_id: ChartId) -> Task<Message> {
        let plan = self
            .charts
            .get(&chart_id)
            .map(|instance| self.plan_liquidation_fetch_for_instance(instance))
            .unwrap_or(LiquidationRequestPlan::Wait);
        match plan {
            LiquidationRequestPlan::Fetch { coin, mark } => {
                self.queue_liquidation_fetch(chart_id, coin, mark)
            }
            LiquidationRequestPlan::Status(message, is_error) => {
                if let Some(instance) = self.charts.get_mut(&chart_id) {
                    instance.liquidation_status = Some((message, is_error));
                    instance.chart.candle_cache.clear();
                }
                Task::none()
            }
            LiquidationRequestPlan::Wait => Task::none(),
        }
    }

    pub(crate) fn clear_liquidation_display(instance: &mut ChartInstance) {
        instance.liquidation_data = None;
        instance.liquidation_fetching = false;
        instance.liquidation_status = None;
        instance.liquidation_pending_key = None;
        instance.chart.liquidation_buckets.clear();
        instance.chart.candle_cache.clear();
    }

    fn plan_liquidation_fetch_for_instance(
        &self,
        instance: &ChartInstance,
    ) -> LiquidationRequestPlan {
        let coin = self.hyperdash_coin_for_symbol(&instance.symbol);
        liquidation_request_plan(LiquidationPlanContext {
            show_liquidations: instance.show_liquidations,
            liquidation_fetching: instance.liquidation_fetching,
            hyperdash_key_missing: self.hyperdash_api_key.is_empty(),
            symbol: &instance.symbol,
            ticker_muted: self.is_ticker_muted(&instance.symbol),
            coin: coin.as_deref(),
            mark: liquidation_mark_from_ctx(
                instance
                    .asset_ctx
                    .as_ref()
                    .and_then(|ctx| ctx.mark_px.as_deref()),
                instance.chart.candles.last().map(|candle| candle.close),
            ),
        })
    }

    fn queue_liquidation_fetch(&mut self, id: ChartId, coin: String, mark: f64) -> Task<Message> {
        self.queue_liquidation_fetch_at(id, coin, mark, Self::now_ms() / 1_000)
    }

    fn queue_liquidation_fetch_at(
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
            move |r| Message::ChartLiquidationLoaded(response_key.clone(), Box::new(r)),
        )
    }

    fn chart_can_accept_liquidation_result(&self, chart_id: ChartId, coin: &str) -> bool {
        self.charts.get(&chart_id).is_some_and(|instance| {
            instance.show_liquidations
                && !instance.symbol.is_empty()
                && !self.is_ticker_muted(&instance.symbol)
                && self
                    .hyperdash_coin_for_symbol(&instance.symbol)
                    .is_some_and(|chart_coin| chart_coin == coin)
        })
    }
}

#[derive(Debug, PartialEq)]
enum LiquidationRequestPlan {
    Fetch { coin: String, mark: f64 },
    Status(String, bool),
    Wait,
}

struct LiquidationPlanContext<'a> {
    show_liquidations: bool,
    liquidation_fetching: bool,
    hyperdash_key_missing: bool,
    symbol: &'a str,
    ticker_muted: bool,
    coin: Option<&'a str>,
    mark: Option<f64>,
}

fn liquidation_request_plan(ctx: LiquidationPlanContext<'_>) -> LiquidationRequestPlan {
    if !ctx.show_liquidations || ctx.liquidation_fetching {
        return LiquidationRequestPlan::Wait;
    }
    if ctx.hyperdash_key_missing {
        return LiquidationRequestPlan::Status(
            "Add HyperDash key in Settings > Integrations".to_string(),
            true,
        );
    }
    if ctx.symbol.is_empty() || ctx.ticker_muted {
        return LiquidationRequestPlan::Wait;
    }
    let Some(coin) = ctx.coin else {
        return LiquidationRequestPlan::Status(
            "Liquidation overlay is only available for perp symbols".to_string(),
            true,
        );
    };
    let Some(mark) = ctx.mark else {
        return LiquidationRequestPlan::Status("LIQ waiting for mark price".to_string(), false);
    };

    LiquidationRequestPlan::Fetch {
        coin: coin.to_string(),
        mark,
    }
}

fn liquidation_mark_from_ctx(mark_px: Option<&str>, fallback_close: Option<f64>) -> Option<f64> {
    mark_px
        .and_then(parse_positive_finite_str)
        .or_else(|| fallback_close.filter(|close| close.is_finite() && *close > 0.0))
}

fn parse_positive_finite_str(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn liquidation_request_key(coin: &str, min: f64, max: f64, timestamp_secs: u64) -> String {
    format!("{coin}:{min:.8}:{max:.8}:{timestamp_secs}")
}

fn liquidation_request_coin(request_key: &str) -> &str {
    request_key.split_once(':').map_or("", |(coin, _)| coin)
}

#[cfg(test)]
mod tests {
    use super::{
        LiquidationPlanContext, LiquidationRequestPlan, liquidation_mark_from_ctx,
        liquidation_request_coin, liquidation_request_key, liquidation_request_plan,
    };

    #[test]
    fn liquidation_mark_parser_rejects_missing_nonpositive_or_nonfinite_values() {
        assert_eq!(liquidation_mark_from_ctx(Some("100.5"), None), Some(100.5));
        assert_eq!(liquidation_mark_from_ctx(None, None), None);
        assert_eq!(liquidation_mark_from_ctx(Some("0"), Some(90.0)), Some(90.0));
        assert_eq!(
            liquidation_mark_from_ctx(Some("-1"), Some(90.0)),
            Some(90.0)
        );
        assert_eq!(
            liquidation_mark_from_ctx(Some("NaN"), Some(90.0)),
            Some(90.0)
        );
        assert_eq!(
            liquidation_mark_from_ctx(Some("bad"), Some(90.0)),
            Some(90.0)
        );
        assert_eq!(liquidation_mark_from_ctx(None, Some(f64::INFINITY)), None);
        assert_eq!(liquidation_mark_from_ctx(None, Some(0.0)), None);
    }

    #[test]
    fn liquidation_request_key_is_stable_for_shared_requests() {
        assert_eq!(
            liquidation_request_key("BTC", 0.0, 161_782.0, 1_778_357_590),
            "BTC:0.00000000:161782.00000000:1778357590"
        );
    }

    #[test]
    fn liquidation_request_coin_reads_shared_request_key() {
        assert_eq!(
            liquidation_request_coin("PURR/USDC:0.00000000:2.00000000:1778357590"),
            "PURR/USDC"
        );
        assert_eq!(liquidation_request_coin("bad-key"), "");
    }

    #[test]
    fn liquidation_plan_waits_when_overlay_is_not_selected() {
        let plan = liquidation_request_plan(LiquidationPlanContext {
            show_liquidations: false,
            liquidation_fetching: false,
            hyperdash_key_missing: true,
            symbol: "BTC",
            ticker_muted: false,
            coin: Some("BTC"),
            mark: Some(100_000.0),
        });

        assert_eq!(plan, LiquidationRequestPlan::Wait);
    }

    #[test]
    fn liquidation_plan_fetches_only_after_overlay_is_selected() {
        let plan = liquidation_request_plan(LiquidationPlanContext {
            show_liquidations: true,
            liquidation_fetching: false,
            hyperdash_key_missing: false,
            symbol: "BTC",
            ticker_muted: false,
            coin: Some("BTC"),
            mark: Some(100_000.0),
        });

        assert_eq!(
            plan,
            LiquidationRequestPlan::Fetch {
                coin: "BTC".to_string(),
                mark: 100_000.0,
            }
        );
    }
}
