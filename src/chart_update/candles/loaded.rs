use crate::api::{
    Candle, candles_have_interval_discontinuity, candles_have_missing_intervals, normalize_candles,
};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::{
    CANDLE_FETCH_MAX_ATTEMPTS, CandleCacheTarget, CandleFetchMode, CandleFetchRequest,
};
use crate::chart_update::is_spot_asset_context_symbol;
use crate::config::ChartBackfillSource;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn apply_chart_cached_candles_loaded(
        &mut self,
        request: CandleFetchRequest,
        target: CandleCacheTarget,
        result: Result<Option<Vec<Candle>>, String>,
    ) -> Task<Message> {
        // Cache hydration is only a boot-time visual aid. It must never satisfy
        // historical pagination or certify freshness.
        if request.mode != CandleFetchMode::Refresh
            || request.source
                != self
                    .chart_backfill_source_for_symbol_timeframe(&request.symbol, request.timeframe)
            || request.read_data_provider_generation != self.read_data_provider_generation
            || (request.source == ChartBackfillSource::Hydromancer
                && !self.hydromancer_key_generation_is_current(request.hydromancer_key_generation))
            || self.symbol_key_is_hidden(&request.symbol)
        {
            return Task::none();
        }

        let Ok(Some(candles)) = result else {
            // The authoritative provider request is already running. A missing
            // or unreadable optional cache must not replace its loading/error
            // state or trigger another retry loop.
            return Task::none();
        };
        let candles = normalize_candles(candles);
        if candles.is_empty() {
            return Task::none();
        }

        let Some(instance) = self.charts.get_mut(&request.chart_id) else {
            return Task::none();
        };
        match target {
            CandleCacheTarget::Primary => {
                let request_matches = instance.symbol == request.symbol
                    && instance.interval == request.timeframe
                    && instance.candle_fetch_request.as_ref() == Some(&request);
                if !request_matches {
                    return Task::none();
                }

                // A websocket candle can arrive while the disk task is still
                // running. Replay the current chart last so cache hydration
                // cannot regress that mutable tail on a duplicate timestamp.
                let live_candles = std::mem::take(&mut instance.chart.candles);
                instance.chart.set_candles(candles);
                instance.chart.merge_candles(live_candles);
            }
            CandleCacheTarget::Secondary => {
                let request_matches = instance.secondary_symbol.as_deref()
                    == Some(request.symbol.as_str())
                    && instance.interval == request.timeframe
                    && instance.secondary_candle_fetch_request.as_ref() == Some(&request);
                if !request_matches {
                    return Task::none();
                }

                let live_candles = instance
                    .chart
                    .secondary_series
                    .as_mut()
                    .map(|series| std::mem::take(&mut series.candles))
                    .unwrap_or_default();
                instance.chart.set_secondary_candles(candles);
                instance.chart.merge_secondary_candles(live_candles);
            }
        }

        Task::none()
    }

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
        let symbol_is_spot =
            self.is_spot_coin(&request.symbol) || is_spot_asset_context_symbol(&request.symbol);
        let is_spot_refresh = request.mode == CandleFetchMode::Refresh && symbol_is_spot;
        let symbol_allows_sparse_intervals = symbol_is_spot || whole_unit_volume;
        let result = result.map(normalize_candles);
        let response_has_interval_gap = result.as_ref().ok().is_some_and(|candles| {
            candle_series_has_unexpected_interval_gap(
                candles,
                request.source,
                &request.symbol,
                request.timeframe,
                symbol_allows_sparse_intervals,
            )
        });
        let received_at_ms = Self::now_ms();
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
                    let ws_updates = std::mem::take(&mut instance.candle_ws_updates_during_fetch);
                    if candles.is_empty() {
                        if instance.chart.candles.is_empty() {
                            let error = format!(
                                "No candle data returned for {} {}",
                                instance.symbol_display, request.timeframe
                            );
                            instance.candle_fetch_error = Some(error.clone());
                            instance.chart.set_error(error);
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
                        if request.mode == CandleFetchMode::Refresh {
                            // A refresh is the authoritative full visible
                            // lookback. Replacing cache-backed history prevents
                            // stale interior buckets from surviving a clean
                            // provider response.
                            instance.chart.set_candles(candles);
                        } else {
                            instance.chart.merge_candles(candles);
                        }
                        // REST was started before any buffered live events. Replay
                        // those events last so the older snapshot cannot regress
                        // the mutable tail on a duplicate timestamp.
                        instance.chart.merge_candles(ws_updates);
                        let merged_has_interval_gap = candle_series_has_unexpected_interval_gap(
                            &instance.chart.candles,
                            request.source,
                            &request.symbol,
                            request.timeframe,
                            symbol_allows_sparse_intervals,
                        );
                        if request.mode == CandleFetchMode::Refresh {
                            instance.candle_history_verified_at_ms = Some(received_at_ms);
                            instance.candle_interval_gap =
                                response_has_interval_gap || merged_has_interval_gap;
                        } else {
                            instance.candle_interval_gap |=
                                response_has_interval_gap || merged_has_interval_gap;
                        }
                        if is_spot_refresh {
                            instance.candle_fetch_error = stale_spot_candle_tail_warning(
                                &instance.chart.candles,
                                request.timeframe,
                                request.end_ms,
                            );
                        }
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
                        instance.candle_ws_updates_during_fetch.clear();
                        let error = redact_sensitive_response_text(&error);
                        if instance.chart.candles.is_empty() {
                            instance.candle_fetch_error = Some(error.clone());
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
        let symbol_allows_sparse_intervals = self.is_spot_coin(&request.symbol)
            || is_spot_asset_context_symbol(&request.symbol)
            || self.is_outcome_coin(&request.symbol);
        let result = result.map(normalize_candles);
        let response_has_interval_gap = result.as_ref().ok().is_some_and(|candles| {
            candle_series_has_unexpected_interval_gap(
                candles,
                request.source,
                &request.symbol,
                request.timeframe,
                symbol_allows_sparse_intervals,
            )
        });
        let received_at_ms = Self::now_ms();
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
                    let ws_updates =
                        std::mem::take(&mut instance.secondary_candle_ws_updates_during_fetch);
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
                        if request.mode == CandleFetchMode::Refresh {
                            instance.chart.set_secondary_candles(candles);
                        } else {
                            instance.chart.merge_secondary_candles(candles);
                        }
                        instance.chart.merge_secondary_candles(ws_updates);
                        let merged_has_interval_gap = instance
                            .chart
                            .secondary_series
                            .as_ref()
                            .is_some_and(|series| {
                                candle_series_has_unexpected_interval_gap(
                                    &series.candles,
                                    request.source,
                                    &request.symbol,
                                    request.timeframe,
                                    symbol_allows_sparse_intervals,
                                )
                            });
                        if request.mode == CandleFetchMode::Refresh {
                            instance.secondary_candle_history_verified_at_ms = Some(received_at_ms);
                            instance.secondary_candle_interval_gap =
                                response_has_interval_gap || merged_has_interval_gap;
                        } else {
                            instance.secondary_candle_interval_gap |=
                                response_has_interval_gap || merged_has_interval_gap;
                        }
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
                        instance.secondary_candle_ws_updates_during_fetch.clear();
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

fn candle_series_has_unexpected_interval_gap(
    candles: &[Candle],
    source: ChartBackfillSource,
    symbol: &str,
    timeframe: crate::timeframe::Timeframe,
    symbol_allows_sparse_intervals: bool,
) -> bool {
    // Calendar months and exchange-session closures do not represent missing
    // cache data. Their varying/closed spans must not produce a permanent
    // warning badge after a successful provider refresh.
    if timeframe == crate::timeframe::Timeframe::Mo1
        || source == ChartBackfillSource::Schwab
        || crate::schwab::is_schwab_symbol_key(symbol)
    {
        return false;
    }
    if symbol_allows_sparse_intervals {
        candles_have_missing_intervals(candles, timeframe.duration_ms())
    } else {
        candles_have_interval_discontinuity(candles, timeframe.duration_ms())
    }
}

fn candle_fetch_error_is_retryable(request: &CandleFetchRequest, error: &str) -> bool {
    match request.source {
        ChartBackfillSource::Hydromancer => !error.contains("Hydromancer API key required"),
        ChartBackfillSource::Schwab => !error.contains("Schwab access token required"),
        ChartBackfillSource::Hyperliquid => true,
    }
}

fn stale_spot_candle_tail_warning(
    candles: &[Candle],
    timeframe: crate::timeframe::Timeframe,
    request_end_ms: u64,
) -> Option<String> {
    let last_close_ms = candles.last()?.close_time;
    let age_ms = request_end_ms.saturating_sub(last_close_ms);
    (age_ms > timeframe.cache_display_max_age_ms()).then(|| {
        format!("Latest spot trade candle is stale for {timeframe}; the market may be inactive")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    fn spot_symbol(key: &str) -> crate::api::ExchangeSymbol {
        crate::api::ExchangeSymbol {
            key: key.to_string(),
            ticker: "SPOT".to_string(),
            category: "spot".to_string(),
            display_name: Some("SPOT/USDC".to_string()),
            keywords: vec!["spot".to_string()],
            asset_index: 10_003,
            collateral_token: Some(crate::api::USDC_TOKEN_INDEX),
            sz_decimals: 2,
            max_leverage: 1,
            only_isolated: false,
            growth_mode: false,
            market_type: crate::api::MarketType::Spot,
            outcome: None,
        }
    }

    #[test]
    fn boot_cache_hydration_keeps_newer_live_tail_and_remains_unverified() {
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
            end_ms: 10_000,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        instance
            .chart
            .set_candles(vec![Candle::test_flat(7_200_000, 110.0)]);
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_cached_candles_loaded(
            request.clone(),
            CandleCacheTarget::Primary,
            Ok(Some(vec![
                Candle::test_flat(3_600_000, 95.0),
                Candle::test_flat(7_200_000, 95.0),
            ])),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles.len(), 2);
        assert_eq!(
            instance.chart.candles.last().map(|candle| candle.close),
            Some(110.0),
            "disk hydration must not overwrite a websocket candle"
        );
        assert_eq!(instance.candle_fetch_request.as_ref(), Some(&request));
        assert!(instance.candle_history_verified_at_ms.is_none());
        assert!(matches!(instance.chart.status, ChartStatus::Loaded));
    }

    #[test]
    fn stale_boot_cache_result_cannot_mutate_a_replaced_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.chart_backfill_source = ChartBackfillSource::Hyperliquid;

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        let stale_request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 10_000,
            attempt: 0,
        };
        let current_request = CandleFetchRequest {
            end_ms: 20_000,
            ..stale_request.clone()
        };
        instance.candle_fetch_request = Some(current_request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_cached_candles_loaded(
            stale_request,
            CandleCacheTarget::Primary,
            Ok(Some(vec![Candle::test_flat(0, 90.0)])),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(instance.chart.candles.is_empty());
        assert_eq!(
            instance.candle_fetch_request.as_ref(),
            Some(&current_request)
        );
        assert!(instance.candle_history_verified_at_ms.is_none());
    }

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
    fn stale_spot_snapshot_remains_loaded_but_is_marked_stale() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.exchange_symbols = vec![spot_symbol("@3")];

        let end_ms = 10 * 3_600_000;
        let mut instance = ChartInstance::new(1, "@3".to_string(), Timeframe::H1);
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "@3".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms,
            attempt: 0,
        };
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let old = Candle::test_ohlcv(
            3_600_000,
            2 * 3_600_000 - 1,
            [100.0, 100.0, 100.0, 100.0],
            1.0,
        );
        let _task = terminal.apply_chart_candles_loaded(request, Ok(vec![old]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(matches!(instance.chart.status, ChartStatus::Loaded));
        assert_eq!(instance.chart.candles.len(), 1);
        assert!(
            instance
                .candle_fetch_error
                .as_deref()
                .is_some_and(|message| message.contains("market may be inactive"))
        );
    }

    #[test]
    fn recent_spot_snapshot_is_not_marked_stale() {
        let candle = Candle::test_ohlcv(
            9 * 3_600_000,
            10 * 3_600_000 - 1,
            [100.0, 100.0, 100.0, 100.0],
            1.0,
        );

        assert!(stale_spot_candle_tail_warning(&[candle], Timeframe::H1, 10 * 3_600_000).is_none());
    }

    #[test]
    fn live_update_received_during_refresh_wins_over_older_rest_snapshot() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 7_200_000,
            attempt: 0,
        };
        let cached = Candle::test_ohlcv(3_600_000, 7_199_999, [90.0, 95.0, 85.0, 90.0], 1.0);
        let rest = Candle::test_ohlcv(3_600_000, 7_199_999, [100.0, 105.0, 95.0, 100.0], 2.0);
        let live = Candle::test_ohlcv(3_600_000, 7_199_999, [100.0, 115.0, 95.0, 110.0], 3.0);
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_candles(vec![cached]);
        instance.candle_fetch_request = Some(request.clone());
        instance.chart.push_candle(live.clone());
        instance.remember_primary_ws_candle(live, 6_000_000);
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(request, Ok(vec![rest]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles.len(), 1);
        assert_eq!(instance.chart.candles[0].close, 110.0);
        assert!(instance.candle_history_verified_at_ms.is_some());
        assert!(instance.candle_ws_updates_during_fetch.is_empty());
    }

    #[test]
    fn provider_refresh_replaces_cache_backed_history_instead_of_stitching_it() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 10_800_000,
            attempt: 0,
        };
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_candles(vec![
            Candle::test_flat(3_600_000, 90.0),
            Candle::test_flat(5_000_000, 150.0),
            Candle::test_flat(7_200_000, 95.0),
        ]);
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(
            request,
            Ok(vec![
                Candle::test_flat(3_600_000, 100.0),
                Candle::test_flat(7_200_000, 101.0),
            ]),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles.len(), 2);
        assert_eq!(
            instance
                .chart
                .candles
                .iter()
                .map(|candle| candle.open_time)
                .collect::<Vec<_>>(),
            vec![3_600_000, 7_200_000]
        );
        assert!(instance.candle_history_verified_at_ms.is_some());
        assert!(!instance.candle_interval_gap);
    }

    #[test]
    fn live_tail_does_not_hide_a_final_history_refresh_error() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 7_200_000,
            attempt: CANDLE_FETCH_MAX_ATTEMPTS - 1,
        };
        let live = Candle::test_flat(3_600_000, 110.0);
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.candle_fetch_request = Some(request.clone());
        instance.chart.set_candles(vec![live.clone()]);
        instance.remember_primary_ws_candle(live, 4_000_000);
        terminal.charts.insert(1, instance);

        let _task =
            terminal.apply_chart_candles_loaded(request, Err("provider unavailable".to_string()));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(matches!(instance.chart.status, ChartStatus::Loaded));
        assert_eq!(instance.chart.candles.len(), 1);
        assert_eq!(
            instance.candle_fetch_error.as_deref(),
            Some("provider unavailable")
        );
        assert!(instance.candle_history_verified_at_ms.is_none());
    }

    #[test]
    fn provider_verified_gap_is_displayed_with_an_explicit_warning() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 14_400_000,
            attempt: 0,
        };
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(3_600_000, 90.0)]);
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(
            request,
            Ok(vec![
                Candle::test_flat(3_600_000, 100.0),
                Candle::test_flat(10_800_000, 110.0),
            ]),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles.len(), 2);
        assert_eq!(instance.chart.candles[0].close, 100.0);
        assert_eq!(instance.chart.candles[1].close, 110.0);
        assert!(instance.candle_fetch_error.is_none());
        assert!(instance.candle_history_verified_at_ms.is_some());
        assert!(instance.candle_interval_gap);
    }

    #[test]
    fn sparse_spot_refresh_keeps_real_prices_and_exposes_missing_intervals() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        terminal.exchange_symbols = vec![spot_symbol("@3")];

        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "@3".to_string(),
            timeframe: Timeframe::H1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: 0,
            end_ms: 14_400_000,
            attempt: 0,
        };
        let mut instance = ChartInstance::new(1, "@3".to_string(), Timeframe::H1);
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);
        let first = Candle::test_ohlcv(3_600_000, 7_199_999, [100.0, 100.0, 100.0, 100.0], 1.0);
        let second = Candle::test_ohlcv(10_800_000, 14_399_999, [110.0, 110.0, 110.0, 110.0], 1.0);

        let _task = terminal.apply_chart_candles_loaded(request, Ok(vec![first, second]));

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert_eq!(instance.chart.candles.len(), 2);
        assert_eq!(instance.chart.candles[1].open, 110.0);
        assert!(instance.candle_interval_gap);
        assert!(instance.candle_history_verified_at_ms.is_some());
    }

    #[test]
    fn calendar_month_spacing_does_not_raise_interval_warning() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();
        let day_ms = 24 * 60 * 60 * 1_000;
        let request = CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::Mo1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: terminal.hydromancer_key_generation,
            start_ms: day_ms,
            end_ms: 63 * day_ms,
            attempt: 0,
        };
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::Mo1);
        instance.candle_fetch_request = Some(request.clone());
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_candles_loaded(
            request,
            Ok(vec![
                Candle::test_flat(day_ms, 100.0),
                Candle::test_flat(32 * day_ms, 110.0),
            ]),
        );

        let instance = terminal.charts.get(&1).expect("chart instance");
        assert!(instance.candle_history_verified_at_ms.is_some());
        assert!(!instance.candle_interval_gap);
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
