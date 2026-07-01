use crate::api::Candle;
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::{CANDLE_FETCH_MAX_ATTEMPTS, CandleFetchMode, CandleFetchRequest};
use crate::config::ChartBackfillSource;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn apply_chart_candles_loaded(
        &mut self,
        request: CandleFetchRequest,
        result: Result<Vec<Candle>, String>,
    ) -> Task<Message> {
        if request.source
            != self.chart_backfill_source_for_symbol_timeframe(&request.symbol, request.timeframe)
        {
            return Task::none();
        }
        if request.read_data_provider_generation != self.read_data_provider_generation {
            return Task::none();
        }
        if request.source == ChartBackfillSource::Hydromancer
            && !self.hydromancer_key_generation_is_current(request.hydromancer_key_generation)
        {
            return Task::none();
        }
        if self.symbol_key_is_hidden(&request.symbol) {
            return Task::none();
        }
        let id = request.chart_id;
        let whole_unit_volume = self.is_outcome_coin(&request.symbol);
        let mut new_cache_data = None;
        let mut remove_cache_data = None;
        let mut retry_request = None;
        let mut fetch_overlays = false;
        let mut continue_older_backfill = false;
        let mut check_viewport_backfill = false;

        if let Some(instance) = self.charts.get_mut(&id) {
            let request_matches = instance.symbol == request.symbol
                && instance.interval == request.timeframe
                && instance.candle_fetch_request.as_ref() == Some(&request);
            if !request_matches {
                return Task::none();
            }
            instance.chart.whole_unit_volume = whole_unit_volume;

            match result {
                Ok(candles) => {
                    instance.candle_fetch_request = None;
                    if candles.is_empty() {
                        if instance.chart.candles.is_empty() {
                            instance.chart.set_error(format!(
                                "No candle data returned for {} {}",
                                instance.symbol_display, request.timeframe
                            ));
                            remove_cache_data = Some((request.symbol.clone(), request.timeframe));
                        } else if request.mode == CandleFetchMode::BackfillOlder {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error = None;
                            instance.candle_backfill_exhausted = true;
                        } else {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error =
                                Some("No fresh candle data returned".to_string());
                        }
                    } else {
                        instance.candle_fetch_error = None;
                        let oldest_before_merge = instance
                            .chart
                            .candles
                            .first()
                            .map(|candle| candle.open_time);
                        instance.chart.merge_candles(candles);
                        let oldest_after_merge = instance
                            .chart
                            .candles
                            .first()
                            .map(|candle| candle.open_time);
                        if request.mode == CandleFetchMode::BackfillOlder {
                            // Only keep paging older if the window actually grew
                            // older. A non-empty page that does not predate the
                            // current oldest candle (provider clamped the range,
                            // returned duplicates, etc.) means we reached the
                            // boundary; stop so we don't re-fetch the same page.
                            if oldest_after_merge < oldest_before_merge {
                                instance.candle_backfill_exhausted = false;
                                continue_older_backfill = true;
                            } else {
                                instance.candle_backfill_exhausted = true;
                            }
                        } else {
                            check_viewport_backfill = true;
                            // Overlays key off the live/visible window, which only
                            // a refresh changes; backfilling old history does not.
                            fetch_overlays = true;
                        }
                        new_cache_data = Some((
                            request.symbol.clone(),
                            request.timeframe,
                            instance.chart.candles.clone(),
                        ));
                    }
                }
                Err(error) => {
                    let next_attempt = request.attempt.saturating_add(1);
                    if next_attempt < CANDLE_FETCH_MAX_ATTEMPTS
                        && candle_fetch_error_is_retryable(&request, &error)
                    {
                        let mut next_request = request.clone();
                        next_request.attempt = next_attempt;
                        if next_request.mode == CandleFetchMode::Refresh {
                            next_request.end_ms = Self::now_ms();
                        }
                        instance.candle_fetch_request = Some(next_request.clone());
                        if instance.chart.candles.is_empty() {
                            instance.chart.status = ChartStatus::Loading;
                        } else {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error = Some(format!(
                                "Retrying candle refresh ({}/{})",
                                next_attempt + 1,
                                CANDLE_FETCH_MAX_ATTEMPTS
                            ));
                        }
                        retry_request = Some(next_request);
                    } else {
                        instance.candle_fetch_request = None;
                        let error = redact_sensitive_response_text(&error);
                        if instance.chart.candles.is_empty() {
                            instance.chart.set_error(error);
                            remove_cache_data = Some((request.symbol.clone(), request.timeframe));
                        } else {
                            instance.chart.status = ChartStatus::Loaded;
                            instance.candle_fetch_error = Some(error);
                        }
                    }
                }
            }
        }

        if let Some(request) = retry_request {
            return Self::fetch_candles_task(
                request,
                self.hydromancer_api_key_for_task(),
                self.schwab.access_token_for_task(),
            );
        }

        if let Some((symbol, tf, new_cache)) = new_cache_data {
            self.sync_chart_position_for(id);
            self.sync_chart_orders_for(id);
            self.sync_chart_trade_markers_for(id);
            self.cache_candles(&symbol, tf, new_cache);
        } else if let Some((symbol, tf)) = remove_cache_data {
            self.remove_cached_candles(&symbol, tf);
        }

        let mut tasks = Vec::new();
        if fetch_overlays {
            tasks.push(self.maybe_fetch_liquidations(id));
            tasks.push(self.maybe_fetch_heatmap(id));
            tasks.push(self.maybe_fetch_chart_funding(id));
        }
        if continue_older_backfill {
            tasks.push(self.continue_older_primary_candle_backfill(id));
        } else if check_viewport_backfill {
            tasks.push(self.maybe_continue_chart_candle_backfill(id));
        }
        if !tasks.is_empty() {
            return Task::batch(tasks);
        }

        Task::none()
    }

    pub(in crate::chart_update) fn apply_chart_secondary_candles_loaded(
        &mut self,
        request: CandleFetchRequest,
        result: Result<Vec<Candle>, String>,
    ) -> Task<Message> {
        if request.source
            != self.chart_backfill_source_for_symbol_timeframe(&request.symbol, request.timeframe)
        {
            return Task::none();
        }
        if request.read_data_provider_generation != self.read_data_provider_generation {
            return Task::none();
        }
        if request.source == ChartBackfillSource::Hydromancer
            && !self.hydromancer_key_generation_is_current(request.hydromancer_key_generation)
        {
            return Task::none();
        }
        if self.symbol_key_is_hidden(&request.symbol) {
            return Task::none();
        }

        let id = request.chart_id;
        let mut new_cache_data = None;
        let mut remove_cache_data = None;
        let mut retry_request = None;
        let mut continue_older_backfill = false;
        let mut check_viewport_backfill = false;

        if let Some(instance) = self.charts.get_mut(&id) {
            let request_matches = instance.secondary_symbol.as_deref()
                == Some(request.symbol.as_str())
                && instance.interval == request.timeframe
                && instance.secondary_candle_fetch_request.as_ref() == Some(&request);
            if !request_matches {
                return Task::none();
            }

            match result {
                Ok(candles) => {
                    instance.secondary_candle_fetch_request = None;
                    if candles.is_empty() {
                        if request.mode == CandleFetchMode::BackfillOlder {
                            instance.secondary_candle_fetch_error = None;
                            instance.secondary_candle_backfill_exhausted = true;
                        } else {
                            instance.secondary_candle_fetch_error =
                                Some("No comparison candle data returned".to_string());
                            remove_cache_data = Some((request.symbol.clone(), request.timeframe));
                        }
                    } else {
                        instance.secondary_candle_fetch_error = None;
                        let oldest_before_merge = instance
                            .chart
                            .secondary_series
                            .as_ref()
                            .and_then(|series| series.candles.first())
                            .map(|candle| candle.open_time);
                        instance.chart.merge_secondary_candles(candles);
                        let oldest_after_merge = instance
                            .chart
                            .secondary_series
                            .as_ref()
                            .and_then(|series| series.candles.first())
                            .map(|candle| candle.open_time);
                        if request.mode == CandleFetchMode::BackfillOlder {
                            // See the primary handler: stop paging when a non-empty
                            // page does not extend the window further back.
                            if oldest_after_merge < oldest_before_merge {
                                instance.secondary_candle_backfill_exhausted = false;
                                continue_older_backfill = true;
                            } else {
                                instance.secondary_candle_backfill_exhausted = true;
                            }
                        } else {
                            check_viewport_backfill = true;
                        }
                        if let Some(series) = instance.chart.secondary_series.as_ref() {
                            new_cache_data = Some((
                                request.symbol.clone(),
                                request.timeframe,
                                series.candles.clone(),
                            ));
                        }
                    }
                }
                Err(error) => {
                    let next_attempt = request.attempt.saturating_add(1);
                    if next_attempt < CANDLE_FETCH_MAX_ATTEMPTS
                        && candle_fetch_error_is_retryable(&request, &error)
                    {
                        let mut next_request = request.clone();
                        next_request.attempt = next_attempt;
                        if next_request.mode == CandleFetchMode::Refresh {
                            next_request.end_ms = Self::now_ms();
                        }
                        instance.secondary_candle_fetch_request = Some(next_request.clone());
                        instance.secondary_candle_fetch_error = Some(format!(
                            "Retrying comparison refresh ({}/{})",
                            next_attempt + 1,
                            CANDLE_FETCH_MAX_ATTEMPTS
                        ));
                        retry_request = Some(next_request);
                    } else {
                        instance.secondary_candle_fetch_request = None;
                        let error = redact_sensitive_response_text(&error);
                        instance.secondary_candle_fetch_error = Some(error);
                    }
                }
            }
        }

        if let Some(request) = retry_request {
            return Self::fetch_secondary_candles_task(
                request,
                self.hydromancer_api_key_for_task(),
                self.schwab.access_token_for_task(),
            );
        }

        if let Some((symbol, tf, new_cache)) = new_cache_data {
            self.cache_candles(&symbol, tf, new_cache);
        } else if let Some((symbol, tf)) = remove_cache_data {
            self.remove_cached_candles(&symbol, tf);
        }

        if continue_older_backfill {
            return self.continue_older_secondary_candle_backfill(id);
        }
        if check_viewport_backfill {
            return self.maybe_continue_chart_candle_backfill(id);
        }

        Task::none()
    }
}

fn candle_fetch_error_is_retryable(request: &CandleFetchRequest, error: &str) -> bool {
    match request.source {
        ChartBackfillSource::Hydromancer => !error.contains("Hydromancer API key required"),
        ChartBackfillSource::Schwab => !error.contains("Schwab access token required"),
        ChartBackfillSource::Hyperliquid => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    #[test]
    fn empty_candle_error_uses_chart_display_name_for_outcome_markets() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        let mut instance = ChartInstance::new(1, "#950".to_string(), Timeframe::H1);
        instance.symbol_display = "YES: Will BTC close green?".to_string();
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "#950".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(request, Ok(Vec::new()));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(instance.chart.whole_unit_volume);
        match &instance.chart.status {
            ChartStatus::Error(message) => {
                assert!(message.contains("YES: Will BTC close green?"), "{message}");
                assert!(!message.contains("#950"), "{message}");
            }
            other => panic!("expected error status, got {other:?}"),
        }
    }

    #[test]
    fn stale_hydromancer_generation_does_not_update_chart_candles() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.hydromancer_key_generation = 2;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 1,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal
            .apply_chart_candles_loaded(request.clone(), Ok(vec![Candle::test_flat(0, 100.0)]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.candle_fetch_request.as_ref(), Some(&request));
        assert!(instance.chart.candles.is_empty());
    }

    #[test]
    fn stale_backfill_source_does_not_update_chart_candles() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task =
            terminal.apply_chart_candles_loaded(request, Ok(vec![Candle::test_flat(0, 100.0)]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(instance.chart.candles.is_empty());
    }

    #[test]
    fn hydromancer_only_timeframe_accepts_hydromancer_source_when_provider_is_hyperliquid() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::S1);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::S1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task =
            terminal.apply_chart_candles_loaded(request, Ok(vec![Candle::test_flat(1_000, 100.0)]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles.len(), 1);
    }

    #[test]
    fn current_primary_candle_error_redacts_chart_error() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: CANDLE_FETCH_MAX_ATTEMPTS - 1,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(
            request,
            Err("candle fetch failed: api_key=chart-secret".to_string()),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        match &instance.chart.status {
            ChartStatus::Error(message) => {
                assert!(message.contains("api_key=<redacted>"));
                assert!(!message.contains("chart-secret"));
            }
            other => panic!("expected error status, got {other:?}"),
        }
    }

    #[test]
    fn stale_hyperliquid_provider_generation_does_not_update_chart_candles() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        terminal.bump_read_data_provider_generation();
        let _task = terminal
            .apply_chart_candles_loaded(request.clone(), Ok(vec![Candle::test_flat(0, 100.0)]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.candle_fetch_request.as_ref(), Some(&request));
        assert!(instance.chart.candles.is_empty());
    }

    #[test]
    fn empty_older_primary_backfill_marks_boundary_without_chart_error() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(1_000, 100.0)]);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::BackfillOlder,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 999,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(request, Ok(Vec::new()));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(matches!(instance.chart.status, ChartStatus::Loaded));
        assert_eq!(instance.chart.candles.len(), 1);
        assert!(instance.candle_fetch_request.is_none());
        assert!(instance.candle_fetch_error.is_none());
        assert!(instance.candle_backfill_exhausted);
    }

    #[test]
    fn non_advancing_older_primary_backfill_marks_boundary_and_stops() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(2_000, 100.0)]);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::BackfillOlder,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_999,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        // Provider returns a non-empty page that does not predate the oldest
        // loaded candle (here a duplicate of it). The window does not grow older,
        // so backfill must stop instead of re-issuing the identical request.
        let _task =
            terminal.apply_chart_candles_loaded(request, Ok(vec![Candle::test_flat(2_000, 105.0)]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(matches!(instance.chart.status, ChartStatus::Loaded));
        assert_eq!(instance.chart.candles.len(), 1);
        assert!(instance.candle_backfill_exhausted);
        // No continuation was queued (the bug would re-fetch the same page).
        assert!(instance.candle_fetch_request.is_none());
    }

    #[test]
    fn retrying_older_primary_backfill_preserves_historical_window() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(1_000, 100.0)]);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::BackfillOlder,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 999,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal
            .apply_chart_candles_loaded(request, Err("transient candle failure".to_string()));

        let retry = terminal
            .charts
            .get(&1)
            .and_then(|instance| instance.candle_fetch_request.as_ref())
            .expect("retry request");
        assert_eq!(retry.mode, CandleFetchMode::BackfillOlder);
        assert_eq!(retry.start_ms, 0);
        assert_eq!(retry.end_ms, 999);
        assert_eq!(retry.attempt, 1);
    }

    #[test]
    fn secondary_candle_load_updates_comparison_series_only() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(1_000, 100.0)]);
        instance.set_secondary_symbol_identity("ETH".to_string(), "ETH".to_string());
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "ETH".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        instance.secondary_candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_secondary_candles_loaded(
            request,
            Ok(vec![Candle::test_flat(2_000, 200.0)]),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles[0].close, 100.0);
        let secondary = instance
            .chart
            .secondary_series
            .as_ref()
            .expect("secondary series");
        assert_eq!(secondary.candles[0].close, 200.0);
        assert!(instance.secondary_candle_fetch_request.is_none());
        assert!(instance.secondary_candle_fetch_error.is_none());
    }

    #[test]
    fn current_secondary_candle_error_redacts_comparison_error() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.set_secondary_symbol_identity("ETH".to_string(), "ETH".to_string());
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "ETH".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: CANDLE_FETCH_MAX_ATTEMPTS - 1,
        };
        instance.secondary_candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_secondary_candles_loaded(
            request,
            Err("comparison fetch failed: signature=chart-secret".to_string()),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        let error = instance
            .secondary_candle_fetch_error
            .as_deref()
            .expect("secondary candle error");
        assert!(error.contains("signature=<redacted>"));
        assert!(!error.contains("chart-secret"));
    }

    #[test]
    fn stale_secondary_candle_load_is_ignored() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.set_secondary_symbol_identity("ETH".to_string(), "ETH".to_string());
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "ETH".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 1_000,
            attempt: 0,
        };
        let stale_request = CandleFetchRequest {
            symbol: "SOL".to_string(),
            ..request.clone()
        };
        instance.secondary_candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_secondary_candles_loaded(
            stale_request,
            Ok(vec![Candle::test_flat(2_000, 200.0)]),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(
            instance.secondary_candle_fetch_request.as_ref(),
            Some(&request)
        );
        assert!(
            instance
                .chart
                .secondary_series
                .as_ref()
                .expect("secondary series")
                .candles
                .is_empty()
        );
    }
}
