use super::super::{CandlestickChart, ChartStatus};
use crate::api::{Candle, is_valid_candle, normalize_candles};

// ---------------------------------------------------------------------------
// Candle Data Lifecycle
// ---------------------------------------------------------------------------

impl CandlestickChart {
    /// Replace all candle data (e.g. after initial fetch or interval change).
    pub fn set_candles(&mut self, candles: Vec<Candle>) {
        self.candles = normalize_candles(candles);
        self.status = if self.candles.is_empty() {
            ChartStatus::Error("No candle data returned".to_string())
        } else {
            ChartStatus::Loaded
        };
        self.candle_cache.clear();
    }

    /// Merge new candles seamlessly, preserving existing ones if applicable.
    pub fn merge_candles(&mut self, mut new_candles: Vec<Candle>) {
        new_candles = normalize_candles(new_candles);
        if self.candles.is_empty() {
            self.candles = new_candles;
        } else if !new_candles.is_empty() {
            let first_new_time = new_candles.first().map(|c| c.open_time).unwrap_or_default();

            self.candles.retain(|c| c.open_time < first_new_time);
            self.candles.append(&mut new_candles);
        }

        if self.candles.len() > 10000 {
            let trim_len = self.candles.len() - 10000;
            self.candles.drain(0..trim_len);
        }

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
            }
        } else {
            self.candles.push(candle);
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
