use crate::api::{self, Candle};
use crate::app_state::TradingTerminal;
use crate::config::ChartBackfillSource;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiCandleFetch, SpaghettiWsCandleContext};
use iced::Task;

impl TradingTerminal {
    fn spaghetti_fetch_is_current(&self, request: &SpaghettiCandleFetch) -> bool {
        request.instance_epoch == self.spaghetti_instance_epoch
            && request.read_data_provider_generation == self.read_data_provider_generation
            && (request.source != ChartBackfillSource::Hydromancer
                || self.hydromancer_key_generation_is_current(request.hydromancer_key_generation))
            && request.source == self.chart_backfill_source
            && !self.symbol_key_is_hidden(&request.symbol)
            && self
                .spaghetti_charts
                .get(&request.chart_id)
                .is_some_and(|inst| {
                    request.timeframe
                        == Self::spaghetti_effective_timeframe_for(
                            inst.interval,
                            inst.canvas.active_session,
                            inst.session_granularity,
                            Self::now_ms(),
                        )
                        && request.session == inst.canvas.active_session
                        && request.session_granularity == inst.session_granularity
                        && inst
                            .canvas
                            .series
                            .iter()
                            .any(|series| series.symbol == request.symbol)
                })
    }

    pub(in crate::spaghetti_update) fn start_spaghetti_fetch(
        &mut self,
        mut request: SpaghettiCandleFetch,
    ) -> Task<Message> {
        if !self.spaghetti_fetch_is_current(&request) {
            return Task::none();
        }
        let api_key = self.hydromancer_api_key_for_task();
        let Some(inst) = self.spaghetti_charts.get_mut(&request.chart_id) else {
            return Task::none();
        };
        let health = inst.health.entry(request.symbol.clone()).or_default();
        if health.pending.as_ref().is_some_and(|pending| {
            pending.timeframe == request.timeframe
                && pending.session == request.session
                && pending.session_granularity == request.session_granularity
                && pending.source == request.source
                && pending.read_data_provider_generation == request.read_data_provider_generation
                && pending.hydromancer_key_generation == request.hydromancer_key_generation
        }) {
            return Task::none();
        }
        if health.verified_ms.is_some()
            && !health.full_refresh_required
            && (health.error.is_some() || health.stream_error.is_some())
            && let Some(last) = inst
                .canvas
                .series
                .iter()
                .find(|series| series.symbol == request.symbol)
                .and_then(|series| series.candles.last())
            && request.end_ms.saturating_sub(last.open_time)
                < request.timeframe.duration_ms().saturating_mul(200)
        {
            request.start_ms = request.start_ms.max(
                last.open_time
                    .saturating_sub(request.timeframe.duration_ms().saturating_mul(2)),
            );
        }
        health.pending = Some(request.clone());
        health.ws_during_fetch.clear();
        let fetch = api::ChartCandleFetchRequest {
            source: request.source,
            hydromancer_api_key: api_key,
            coin: request.symbol.clone(),
            interval: request.timeframe.api_str().to_string(),
            start_time: request.start_ms,
            end_time: request.end_ms,
            policy: api::CandleFetchPolicy::NetworkOnly,
        };
        Task::perform(api::fetch_chart_backfill_candles(fetch), move |result| {
            Message::SpaghettiCandlesLoaded(request.clone(), result)
        })
    }

    pub(in crate::spaghetti_update) fn apply_spaghetti_candles_loaded(
        &mut self,
        request: SpaghettiCandleFetch,
        result: Result<Vec<Candle>, String>,
    ) -> Task<Message> {
        if !self.spaghetti_fetch_is_current(&request) {
            return Task::none();
        }
        let mut cache = None;
        if let Some(inst) = self.spaghetti_charts.get_mut(&request.chart_id) {
            let health = inst.health.entry(request.symbol.clone()).or_default();
            if health.pending.as_ref() != Some(&request) {
                return Task::none();
            }
            health.pending = None;
            let Some(series) = inst
                .canvas
                .series
                .iter_mut()
                .find(|series| series.symbol == request.symbol)
            else {
                return Task::none();
            };
            match result {
                Ok(candles) if !candles.is_empty() => {
                    // Provider history is authoritative; live arrivals after dispatch
                    // win over its forming candle. Keep earlier loaded history.
                    let first = candles.first().map_or(request.start_ms, |c| c.open_time);
                    series.candles.retain(|c| c.open_time < first);
                    series.candles.extend(candles);
                    series.candles.append(&mut health.ws_during_fetch);
                    series.candles = api::normalize_candles(std::mem::take(&mut series.candles));
                    if series.candles.len() > 10_000 {
                        series.candles.drain(..series.candles.len() - 10_000);
                    }
                    series.loaded = !series.candles.is_empty();
                    health.verified_ms = Some(Self::now_ms());
                    health.full_refresh_required = false;
                    health.error = None;
                    health.failures = 0;
                    health.next_retry_ms = Self::now_ms().saturating_add(60_000);
                    cache = Some(series.candles.clone());
                }
                result => {
                    health.error = Some(
                        result
                            .err()
                            .unwrap_or_else(|| "No candle history returned".to_string()),
                    );
                    health.failures = health.failures.saturating_add(1);
                    health.next_retry_ms =
                        Self::now_ms().saturating_add(30_000 * (1u64 << health.failures.min(4)));
                    health.ws_during_fetch.clear();
                    // Retain the last good history and the live subscription.
                }
            }
            Self::refresh_spaghetti_session_anchor(inst);
            inst.canvas.cache.clear();
        }
        if let Some(candles) = cache {
            self.cache_candles(&request.symbol, request.timeframe, candles);
        }
        Task::none()
    }

    pub(in crate::spaghetti_update) fn apply_spaghetti_ws_candle_update(
        &mut self,
        context: SpaghettiWsCandleContext,
        candle: Candle,
    ) -> Task<Message> {
        if !self.spaghetti_ws_candle_context_is_current(&context) || !api::is_valid_candle(&candle)
        {
            return Task::none();
        }
        let mut gap = false;
        if let Some(inst) = self.spaghetti_charts.get_mut(&context.chart_id) {
            gap = inst
                .canvas
                .series
                .iter()
                .find(|series| series.symbol == context.symbol)
                .and_then(|series| series.candles.last())
                .is_some_and(|last| {
                    context.timeframe != crate::timeframe::Timeframe::Mo1
                        && api::open_time_starts_after_gap(
                            last.open_time,
                            candle.open_time,
                            context.timeframe.duration_ms(),
                        )
                });
            let health = inst.health.entry(context.symbol.clone()).or_default();
            health.last_ws_ms = Some(Self::now_ms());
            health.stream_error = None;
            if gap {
                health.full_refresh_required = true;
                health.error = Some("Candle history gap · verifying history".to_string());
            }
            if health.pending.is_some() {
                if let Some(existing) = health
                    .ws_during_fetch
                    .iter_mut()
                    .find(|c| c.open_time == candle.open_time)
                {
                    *existing = candle.clone();
                } else {
                    health.ws_during_fetch.push(candle.clone());
                }
                if health.ws_during_fetch.len() > 10_000 {
                    health.ws_during_fetch.remove(0);
                }
            }
            inst.canvas.push_candle(&context.symbol, candle);
            Self::refresh_spaghetti_session_anchor(inst);
        }
        if gap {
            return self.repair_spaghetti_series(context.chart_id, &context.symbol, false);
        }
        Task::none()
    }

    pub(in crate::spaghetti_update) fn spaghetti_ws_candle_context_is_current(
        &self,
        context: &SpaghettiWsCandleContext,
    ) -> bool {
        if context.instance_epoch != self.spaghetti_instance_epoch
            || !self.market_stream_source_is_current(context.source_context)
            || self.symbol_key_is_hidden(&context.symbol)
        {
            return false;
        }
        let Some(inst) = self.spaghetti_charts.get(&context.chart_id) else {
            return false;
        };
        Self::spaghetti_effective_timeframe_for(
            inst.interval,
            inst.canvas.active_session,
            inst.session_granularity,
            Self::now_ms(),
        ) == context.timeframe
            && inst.canvas.active_session == context.session
            && inst.session_granularity == context.session_granularity
            && inst
                .canvas
                .series
                .iter()
                .any(|series| series.symbol == context.symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spaghetti::Series;
    use crate::spaghetti_state::SpaghettiChartInstance;
    use crate::timeframe::Timeframe;

    fn setup() -> (TradingTerminal, SpaghettiCandleFetch) {
        let mut terminal = TradingTerminal::boot().0;
        terminal.spaghetti_charts.clear();
        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.interval = Timeframe::M1;
        for symbol in ["BTC", "ETH"] {
            instance.canvas.series.push(Series {
                symbol: symbol.into(),
                display: symbol.into(),
                candles: vec![Candle::test_flat(60_000, 100.0)],
                color: iced::Color::BLACK,
                loaded: true,
            });
        }
        terminal.spaghetti_charts.insert(7, instance);
        let request = SpaghettiCandleFetch {
            request_id: 1,
            start_ms: 60_000,
            end_ms: 120_000,
            chart_id: 7,
            instance_epoch: terminal.spaghetti_instance_epoch,
            symbol: "BTC".into(),
            timeframe: Timeframe::M1,
            source: terminal.chart_backfill_source,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            session: None,
            session_granularity: None,
        };
        (terminal, request)
    }
    fn context(terminal: &TradingTerminal) -> SpaghettiWsCandleContext {
        SpaghettiWsCandleContext {
            chart_id: 7,
            instance_epoch: terminal.spaghetti_instance_epoch,
            symbol: "BTC".into(),
            timeframe: Timeframe::M1,
            source_context: terminal.market_data_source_context(),
            session: None,
            session_granularity: None,
        }
    }

    #[test]
    fn failed_history_retains_data_and_live_updates_then_retries_only_failed_series() {
        let (mut terminal, request) = setup();
        let _ = terminal.start_spaghetti_fetch(request.clone());
        let _ = terminal.apply_spaghetti_candles_loaded(request, Err("HTTP 429".into()));
        assert!(terminal.spaghetti_charts[&7].canvas.series[0].loaded);
        assert_eq!(
            terminal.spaghetti_charts[&7].canvas.series[0].candles.len(),
            1
        );
        let _ = terminal.apply_spaghetti_ws_candle_update(
            context(&terminal),
            Candle::test_flat(120_000, 110.0),
        );
        assert_eq!(
            terminal.spaghetti_charts[&7].canvas.series[0]
                .candles
                .last()
                .unwrap()
                .close,
            110.0
        );
        assert!(!terminal.spaghetti_charts[&7].health.contains_key("ETH"));
        terminal
            .spaghetti_charts
            .get_mut(&7)
            .unwrap()
            .health
            .get_mut("BTC")
            .unwrap()
            .next_retry_ms = 0;
        assert_eq!(terminal.repair_spaghetti_series(7, "BTC", false).units(), 1);
        assert_eq!(terminal.repair_spaghetti_series(7, "BTC", false).units(), 0);
        assert_eq!(
            terminal.spaghetti_charts[&7].canvas.series[1].candles.len(),
            1
        );
    }

    #[test]
    fn live_candle_wins_over_delayed_rest_and_duplicate_requests_are_suppressed() {
        let (mut terminal, request) = setup();
        assert_eq!(terminal.start_spaghetti_fetch(request.clone()).units(), 1);
        let mut duplicate = request.clone();
        duplicate.request_id = 2;
        assert_eq!(terminal.start_spaghetti_fetch(duplicate.clone()).units(), 0);
        let _ = terminal
            .apply_spaghetti_ws_candle_update(context(&terminal), Candle::test_flat(60_000, 120.0));
        let _ = terminal
            .apply_spaghetti_candles_loaded(duplicate, Ok(vec![Candle::test_flat(60_000, 80.0)]));
        assert!(
            terminal.spaghetti_charts[&7].health["BTC"]
                .pending
                .is_some()
        );
        let _ = terminal
            .apply_spaghetti_candles_loaded(request, Ok(vec![Candle::test_flat(60_000, 105.0)]));
        assert_eq!(
            terminal.spaghetti_charts[&7].canvas.series[0]
                .candles
                .last()
                .unwrap()
                .close,
            120.0
        );
        assert!(
            terminal.spaghetti_charts[&7].health["BTC"]
                .pending
                .is_none()
        );
    }

    #[test]
    fn live_data_can_bootstrap_a_series_before_history_succeeds() {
        let (mut terminal, _) = setup();
        let instance = terminal.spaghetti_charts.get_mut(&7).unwrap();
        instance.canvas.series[0].candles.clear();
        instance.canvas.series[0].loaded = false;
        let _ = terminal
            .apply_spaghetti_ws_candle_update(context(&terminal), Candle::test_flat(60_000, 120.0));
        assert!(terminal.spaghetti_charts[&7].canvas.series[0].loaded);
    }
    #[test]
    fn provider_change_invalidates_comparison_pending_state() {
        let (mut terminal, request) = setup();
        let _ = terminal.start_spaghetti_fetch(request.clone());
        terminal.bump_read_data_provider_generation();
        let _ = terminal.reload_chart_backfills_for_source_change();
        assert!(terminal.spaghetti_charts[&7].health.is_empty());
        let _ = terminal
            .apply_spaghetti_candles_loaded(request, Ok(vec![Candle::test_flat(60_000, 120.0)]));
        assert!(
            terminal.spaghetti_charts[&7].canvas.series[0]
                .candles
                .is_empty()
        );
    }

    #[test]
    fn detaching_a_live_only_series_still_loads_its_missing_history() {
        let (mut terminal, request) = setup();
        terminal.next_spaghetti_id = 8;
        terminal
            .spaghetti_charts
            .get_mut(&7)
            .expect("source")
            .canvas
            .series
            .truncate(1);
        let _ = terminal.start_spaghetti_fetch(request);
        let _ = terminal
            .apply_spaghetti_ws_candle_update(context(&terminal), Candle::test_flat(60_000, 120.0));
        let task = terminal.open_detached_spaghetti_window(7);
        assert_eq!(task.units(), 2, "open window and load unverified history");
        assert!(
            terminal.spaghetti_charts[&8].health["BTC"]
                .pending
                .is_none()
        );
    }
}
