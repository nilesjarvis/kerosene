use crate::api::{self, Candle};
use crate::app_state::TradingTerminal;
use crate::chart_indicator::{average_history_span, enabled_average_period};
use crate::config::{ChartBackfillSource, EmaCloudTimeframe, MacroIndicatorsConfig};
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::Task;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const MINUTE_MS: u64 = 60_000;

pub(super) fn fixed_average_periods(
    config: &MacroIndicatorsConfig,
) -> impl Iterator<Item = (Timeframe, usize)> + '_ {
    EmaCloudTimeframe::ALL.into_iter().filter_map(|source| {
        Some((
            source.source()?,
            enabled_average_period(config, source.group())?,
        ))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacroCandleRequest {
    pub(crate) request_id: u64,
    pub(crate) symbol: String,
    pub(crate) timeframe: Timeframe,
    pub(crate) period: usize,
    pub(crate) end_ms: u64,
    pub(crate) start_ms: u64,
}

impl MacroCandleRequest {
    fn history_start(&self) -> u64 {
        self.end_ms
            .saturating_sub(average_history_span(self.timeframe, self.period))
    }

    fn fetch(self) -> Task<Message> {
        let request = api::ChartCandleFetchRequest {
            source: ChartBackfillSource::Hyperliquid,
            hydromancer_api_key: Default::default(),
            coin: self.symbol.clone(),
            interval: self.timeframe.api_str().to_string(),
            start_time: self.start_ms,
            end_time: self.end_ms,
            policy: api::CandleFetchPolicy::NetworkOnly,
        };
        Task::perform(api::fetch_chart_backfill_candles(request), move |result| {
            Message::MacroCandlesLoaded(self.clone(), result)
        })
    }

    fn load_cache(self) -> Task<Message> {
        let request = self.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    crate::api_cache::load_macro_candle_history(&request.symbol, request.timeframe)
                        .ok()
                        .flatten()
                        .unwrap_or_default()
                })
                .await
                .unwrap_or_default()
            },
            move |candles| Message::MacroCandlesCached(self.clone(), candles),
        )
    }
}

#[derive(Default)]
pub(crate) struct MacroHistoryEntry {
    candles: Arc<[Candle]>,
    pending: Option<MacroCandleRequest>,
    /// Period covered by the latest successful provider request.
    fetched_period: usize,
    last_full_fetch_ms: u64,
    failures: u8,
    next_fetch_ms: u64,
}

impl MacroHistoryEntry {
    fn use_cached_prefix(&self, request: &mut MacroCandleRequest) {
        if self.candles.is_empty() {
            return;
        }
        // A cache written by a shorter-period chart may cover only the tail.
        // Allow one bucket at the left edge because the request starts mid-bar.
        let full_start = request.history_start();
        let has_warmup = self.candles.len() >= request.period;
        // Newly listed markets can legitimately have fewer observations than
        // the period. Poll their tail and occasionally recheck the full range.
        let verified = self.fetched_period >= request.period
            && (has_warmup
                || request.end_ms.saturating_sub(self.last_full_fetch_ms) < 6 * 60 * MINUTE_MS);
        if verified
            || (has_warmup
                && self.candles[0].open_time
                    <= full_start.saturating_add(request.timeframe.duration_ms()))
        {
            // Re-read the last two observations, including any candle which
            // was still open when the previous provider snapshot was taken.
            let overlap = &self.candles[self.candles.len().saturating_sub(2)];
            request.start_ms = full_start.max(overlap.open_time);
        }
    }

    fn queue(
        &mut self,
        symbol: &str,
        timeframe: Timeframe,
        period: usize,
        now_ms: u64,
    ) -> Option<Task<Message>> {
        if self.pending.is_some()
            || (now_ms < self.next_fetch_ms && (self.failures > 0 || self.fetched_period >= period))
        {
            return None;
        }
        static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);
        let mut request = MacroCandleRequest {
            request_id: NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
            symbol: symbol.to_owned(),
            timeframe,
            period,
            end_ms: now_ms,
            start_ms: now_ms.saturating_sub(average_history_span(timeframe, period)),
        };
        self.use_cached_prefix(&mut request);
        self.pending = Some(request.clone());
        Some(if self.candles.is_empty() && self.failures == 0 {
            request.load_cache()
        } else {
            request.fetch()
        })
    }

    fn failed(&mut self, now_ms: u64) {
        self.failures = self.failures.saturating_add(1);
        let delay = MINUTE_MS * (1 << self.failures.saturating_sub(1).min(4));
        self.next_fetch_ms = now_ms.saturating_add(delay);
    }
}

impl TradingTerminal {
    /// One history request serves all lines and clouds using this symbol and interval.
    pub(crate) fn queue_macro_history(&mut self, now_ms: u64) -> Vec<Task<Message>> {
        let mut open_symbols = HashSet::new();
        let mut demand = HashMap::new();
        for (id, chart) in &self.charts {
            if chart.symbol.is_empty()
                || chart.symbol == "@0"
                || self.symbol_key_is_hidden(&chart.symbol)
            {
                continue;
            }
            open_symbols.insert(chart.symbol.as_str());
            for (tf, period) in fixed_average_periods(&chart.macro_indicators) {
                let (max_period, charts) = demand
                    .entry((chart.symbol.clone(), tf))
                    .or_insert((0, Vec::new()));
                *max_period = period.max(*max_period);
                charts.push(*id);
            }
        }
        // Turning off the last overlay suspends its reads. Keep the loaded
        // history while a pane for this symbol is open so it can be reused.
        self.chart_macro_history.retain(|key, entry| {
            if demand.contains_key(key) {
                return true;
            }
            entry.pending = None;
            !entry.candles.is_empty() && open_symbols.contains(key.0.as_str())
        });
        let mut tasks = Vec::new();
        for ((symbol, tf), (period, charts)) in demand {
            let entry = self
                .chart_macro_history
                .entry((symbol.clone(), tf))
                .or_default();
            for id in charts {
                if let Some(chart) = self.charts.get_mut(&id) {
                    chart.chart.set_macro_history(tf, &entry.candles);
                }
            }
            tasks.extend(entry.queue(&symbol, tf, period, now_ms));
        }
        tasks
    }

    /// Only outstanding work for a still-allowed symbol may publish results.
    fn pending_macro_history(
        &mut self,
        request: &MacroCandleRequest,
    ) -> Option<&mut MacroHistoryEntry> {
        if self.symbol_key_is_hidden(&request.symbol) {
            return None;
        }
        self.chart_macro_history
            .get_mut(&(request.symbol.clone(), request.timeframe))
            .filter(|entry| entry.pending.as_ref() == Some(request))
    }

    pub(crate) fn apply_cached_macro_candles(
        &mut self,
        mut request: MacroCandleRequest,
        mut candles: Vec<Candle>,
    ) -> Task<Message> {
        let Some(entry) = self.pending_macro_history(&request) else {
            return Task::none();
        };
        candles.retain(|c| c.close_time >= request.start_ms && c.close_time <= request.end_ms);
        entry.candles = candles.into();
        entry.use_cached_prefix(&mut request);
        entry.pending = Some(request.clone());
        let mut tasks = self.queue_macro_history(Self::now_ms());
        if self.pending_macro_history(&request).is_some() {
            tasks.push(request.fetch());
        }
        Task::batch(tasks)
    }

    pub(crate) fn apply_macro_candles(
        &mut self,
        mut request: MacroCandleRequest,
        result: Result<Vec<Candle>, String>,
        now_ms: u64,
    ) -> Task<Message> {
        let Some(entry) = self.pending_macro_history(&request) else {
            return Task::none();
        };
        entry.pending = None;
        let Ok(candles) = result else {
            entry.failed(now_ms);
            return Task::none();
        };
        let Some((first, last)) = candles.first().zip(candles.last()) else {
            entry.failed(now_ms);
            return Task::none();
        };

        if entry
            .candles
            .last()
            .is_some_and(|previous| last.open_time < previous.open_time)
        {
            entry.failed(now_ms);
            return Task::none();
        }
        let full_start = request.history_start();
        let refresh_at = last.close_time.saturating_add(1);
        let overlap = entry
            .candles
            .binary_search_by_key(&first.open_time, |c| c.open_time);
        if request.start_ms > full_start && overlap.is_err() {
            // A clamped tail cannot establish continuity. Keep the displayed
            // history while a full-range request resolves the gap.
            request.start_ms = full_start;
            entry.pending = Some(request.clone());
            return request.fetch();
        }
        if request.start_ms == full_start {
            entry.last_full_fetch_ms = now_ms;
        }
        let merged = if let Ok(end) = overlap {
            // Even a full-range response can be shorter than the cache. Keep
            // the closed prefix when the provider supplies a matching bucket.
            let begin = entry.candles[..end].partition_point(|c| c.close_time < full_start);
            let mut merged = entry.candles[begin..end].to_vec();
            merged.extend(candles);
            merged
        } else {
            candles
        };
        entry.candles = merged.into();
        entry.fetched_period = request.period;
        entry.failures = 0;
        // An illiquid source can legitimately end in an already closed candle.
        // Bound both its poll rate and how long a provisional close stays frozen.
        let poll_at = now_ms.saturating_add(15 * MINUTE_MS);
        entry.next_fetch_ms = if refresh_at > now_ms {
            refresh_at.clamp(now_ms.saturating_add(MINUTE_MS), poll_at)
        } else {
            poll_at
        };
        Task::batch(self.queue_macro_history(now_ms))
    }
}

#[cfg(test)]
mod tests;
