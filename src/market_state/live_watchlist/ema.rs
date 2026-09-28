use crate::api::WatchlistEmaSample;
use crate::app_state::TradingTerminal;
use crate::config::{LiveWatchlistColumn, LiveWatchlistEmaConfig};
use crate::message::Message;
use iced::Task;
use std::collections::{HashMap, HashSet};

const REFRESH_MS: u64 = 60_000;
const MAX_IN_FLIGHT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LiveWatchlistEmaKey {
    pub(crate) symbol: String,
    pub(crate) settings: LiveWatchlistEmaConfig,
}

#[derive(Default)]
pub(crate) struct LiveWatchlistEmaState {
    entries: HashMap<LiveWatchlistEmaKey, EmaEntry>,
    next_request_id: u64,
}

#[derive(Default)]
struct EmaEntry {
    pending: Option<u64>,
    attempted_at_ms: Option<u64>,
    sample: Option<WatchlistEmaSample>,
    error: Option<String>,
}

impl LiveWatchlistEmaState {
    fn plan(
        &mut self,
        needed: &HashSet<LiveWatchlistEmaKey>,
        now_ms: u64,
    ) -> Vec<(LiveWatchlistEmaKey, u64)> {
        // Keep obsolete in-flight requests counted until they finish; rapid
        // setting edits must not bypass the concurrency limit.
        self.entries
            .retain(|key, entry| needed.contains(key) || entry.pending.is_some());
        let in_flight = self
            .entries
            .values()
            .filter(|entry| entry.pending.is_some())
            .count();
        let mut keys: Vec<_> = needed.iter().cloned().collect();
        // Load untouched symbols before refreshing old ones, including when a
        // large watchlist takes longer than one refresh interval to drain.
        keys.sort_by(|a, b| {
            let last_attempt = |key| {
                self.entries
                    .get(key)
                    .and_then(|entry| entry.attempted_at_ms)
            };
            (
                last_attempt(a),
                &a.symbol,
                &a.settings.timeframe,
                a.settings.period,
            )
                .cmp(&(
                    last_attempt(b),
                    &b.symbol,
                    &b.settings.timeframe,
                    b.settings.period,
                ))
        });
        let mut requests = Vec::new();
        for key in keys {
            if requests.len() >= MAX_IN_FLIGHT.saturating_sub(in_flight) {
                break;
            }
            let entry = self.entries.entry(key.clone()).or_default();
            let refresh_ms = if entry
                .sample
                .as_ref()
                .is_some_and(|sample| sample.candle_has_closed(now_ms))
            {
                15_000
            } else {
                REFRESH_MS
            };
            if entry.pending.is_some()
                || entry
                    .attempted_at_ms
                    .is_some_and(|last| now_ms.saturating_sub(last) < refresh_ms)
            {
                continue;
            }
            self.next_request_id = self.next_request_id.saturating_add(1);
            entry.pending = Some(self.next_request_id);
            entry.attempted_at_ms = Some(now_ms);
            requests.push((key, self.next_request_id));
        }
        requests
    }

    fn apply(
        &mut self,
        key: &LiveWatchlistEmaKey,
        request_id: u64,
        result: Result<WatchlistEmaSample, String>,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(key) else {
            return false;
        };
        if entry.pending != Some(request_id) {
            return false;
        }
        entry.pending = None;
        match result {
            Ok(sample) => {
                entry.sample = Some(sample);
                entry.error = None;
            }
            Err(error) => {
                entry.sample = None;
                entry.error = Some(crate::helpers::redact_sensitive_response_text(&error));
            }
        }
        true
    }

    pub(crate) fn value(
        &self,
        key: &LiveWatchlistEmaKey,
        mid: Option<f64>,
        now_ms: u64,
    ) -> (Option<f64>, Option<String>) {
        let Some(entry) = self.entries.get(key) else {
            return (None, Some("Loading EMA…".to_string()));
        };
        if let Some(sample) = &entry.sample {
            let distance = sample.distance(mid, now_ms);
            return (
                distance,
                distance
                    .is_none()
                    .then(|| "Waiting for fresh EMA / price data".to_string()),
            );
        }
        (
            None,
            Some(
                entry
                    .error
                    .clone()
                    .unwrap_or_else(|| "Loading EMA…".to_string()),
            ),
        )
    }
}

impl TradingTerminal {
    fn live_watchlist_ema_keys(&self) -> HashSet<LiveWatchlistEmaKey> {
        let open_ids = self.open_live_watchlist_ids();
        self.live_watchlists
            .values()
            .filter(|watchlist| {
                open_ids.contains(&watchlist.id)
                    && watchlist
                        .visible_columns
                        .contains(&LiveWatchlistColumn::EmaDistance)
            })
            .flat_map(|watchlist| {
                watchlist
                    .symbols
                    .iter()
                    .filter(|symbol| {
                        !(self.symbol_key_is_hidden(symbol)
                            || self.symbols_loading && symbol.as_str() == "@0")
                            && self
                                .exchange_symbol_for_key(symbol)
                                .is_none_or(|metadata| metadata.is_user_selectable_market())
                    })
                    .map(|symbol| LiveWatchlistEmaKey {
                        symbol: symbol.clone(),
                        settings: watchlist.ema.clone(),
                    })
            })
            .collect()
    }

    pub(crate) fn request_live_watchlist_ema_refresh(&mut self) -> Task<Message> {
        let needed = self.live_watchlist_ema_keys();
        let requests = self.live_watchlist_ema.plan(&needed, Self::now_ms());
        Task::batch(requests.into_iter().map(|(key, request_id)| {
            Task::perform(
                crate::api::fetch_watchlist_ema(key.symbol.clone(), key.settings.clone()),
                move |result| Message::LiveWatchlistEmaLoaded(key.clone(), request_id, result),
            )
        }))
    }

    pub(crate) fn apply_live_watchlist_ema_loaded(
        &mut self,
        key: LiveWatchlistEmaKey,
        request_id: u64,
        result: Result<WatchlistEmaSample, String>,
    ) -> Task<Message> {
        if !self.live_watchlist_ema.apply(&key, request_id, result) {
            return Task::none();
        }
        let task = self.request_live_watchlist_ema_refresh();
        self.refresh_live_watchlist_row_caches();
        task
    }
}

#[cfg(test)]
mod tests;
