use super::super::{CandlestickChart, ChartStatus};
use crate::api::{Candle, is_valid_candle, normalize_candles};
use crate::chart::model::SecondarySeries;

// ---------------------------------------------------------------------------
// Candle Data Lifecycle
// ---------------------------------------------------------------------------

pub(crate) const MAX_CHART_CANDLES: usize = 10_000;

impl CandlestickChart {
    /// Replace all candle data (e.g. after initial fetch or interval change).
    pub fn set_candles(&mut self, candles: Vec<Candle>) {
        self.candles = normalize_candles(candles);
        trim_to_max_chart_candles(&mut self.candles);
        self.status = if self.candles.is_empty() {
            ChartStatus::Error("No candle data returned".to_string())
        } else {
            ChartStatus::Loaded
        };
        self.candle_cache.clear();
    }

    /// Merge new candles seamlessly, preserving existing ones if applicable.
    pub fn merge_candles(&mut self, new_candles: Vec<Candle>) {
        merge_into_candle_series(&mut self.candles, new_candles);
        self.status = if self.candles.is_empty() {
            ChartStatus::Error("No candle data returned".to_string())
        } else {
            ChartStatus::Loaded
        };
        self.candle_cache.clear();
    }

    /// Append or update the latest candle from a real-time feed.
    pub fn push_candle(&mut self, candle: Candle) {
        if !is_valid_candle(&candle) {
            return;
        }
        if let Some(last) = self.candles.last_mut() {
            if last.open_time == candle.open_time {
                *last = candle;
            } else {
                self.candles.push(candle);
                trim_to_max_chart_candles(&mut self.candles);
            }
        } else {
            self.candles.push(candle);
        }
        self.candle_cache.clear();
    }

    pub(crate) fn set_secondary_series_identity(
        &mut self,
        symbol_key: String,
        symbol_label: String,
    ) {
        let changed = self.secondary_series.as_ref().is_none_or(|series| {
            series.symbol_key != symbol_key || series.symbol_label != symbol_label
        });
        if changed {
            self.secondary_series = Some(SecondarySeries {
                symbol_key,
                symbol_label,
                candles: Vec::new(),
            });
            self.candle_cache.clear();
        }
    }

    pub(crate) fn clear_secondary_series(&mut self) {
        if self.secondary_series.is_some() {
            self.secondary_series = None;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_secondary_candles(&mut self, candles: Vec<Candle>) {
        let Some(series) = self.secondary_series.as_mut() else {
            return;
        };
        series.candles = normalize_candles(candles);
        trim_to_max_chart_candles(&mut series.candles);
        self.candle_cache.clear();
    }

    pub(crate) fn merge_secondary_candles(&mut self, new_candles: Vec<Candle>) {
        let Some(series) = self.secondary_series.as_mut() else {
            return;
        };
        merge_into_candle_series(&mut series.candles, new_candles);
        self.candle_cache.clear();
    }

    pub(crate) fn push_secondary_candle(&mut self, candle: Candle) {
        if !is_valid_candle(&candle) {
            return;
        }
        let Some(series) = self.secondary_series.as_mut() else {
            return;
        };
        if let Some(last) = series.candles.last_mut() {
            if last.open_time == candle.open_time {
                *last = candle;
            } else {
                series.candles.push(candle);
                trim_to_max_chart_candles(&mut series.candles);
            }
        } else {
            series.candles.push(candle);
        }
        self.candle_cache.clear();
    }

    pub fn set_error(&mut self, msg: String) {
        self.status = ChartStatus::Error(msg);
        self.candle_cache.clear();
    }

    pub(crate) fn clear_macro_candles(&mut self) {
        self.daily_candles.clear();
        self.weekly_candles.clear();
        self.monthly_candles.clear();
        self.candle_cache.clear();
    }
}

/// Merge freshly fetched candles into an existing series and bound it: append,
/// then `normalize_candles` (sort by `open_time`, drop invalid, dedupe keeping
/// the later element so a newly fetched candle wins on a duplicate timestamp),
/// then trim to `MAX_CHART_CANDLES`. Both inputs are concatenated and sorted
/// once — `new_candles` is intentionally not pre-normalized.
fn merge_into_candle_series(existing: &mut Vec<Candle>, new_candles: Vec<Candle>) {
    if existing.is_empty() {
        *existing = normalize_candles(new_candles);
    } else if !new_candles.is_empty() {
        let mut merged = std::mem::take(existing);
        merged.reserve(new_candles.len());
        merged.extend(new_candles);
        *existing = normalize_candles(merged);
    }

    trim_to_max_chart_candles(existing);
}

fn trim_to_max_chart_candles(candles: &mut Vec<Candle>) {
    if candles.len() > MAX_CHART_CANDLES {
        let trim_len = candles.len() - MAX_CHART_CANDLES;
        candles.drain(0..trim_len);
    }
}
