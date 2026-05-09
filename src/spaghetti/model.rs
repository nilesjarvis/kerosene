use super::Session;
use crate::api::{Candle, is_valid_candle};

use iced::widget::canvas;
use iced::{Color, Theme};

/// Color palette for series lines.
pub fn series_colors(theme: &Theme) -> Vec<Color> {
    vec![
        theme.palette().success,                         // green
        theme.palette().danger,                          // red
        theme.palette().primary,                         // blue
        theme.extended_palette().secondary.base.color,   // yellow
        theme.extended_palette().secondary.strong.color, // purple
        theme.extended_palette().primary.weak.color,     // pink
        theme.extended_palette().primary.strong.color,   // cyan
        theme.extended_palette().danger.weak.color,      // orange
    ]
}

/// A single series in the comparison chart.
pub struct Series {
    pub symbol: String,
    pub display: String,
    pub candles: Vec<Candle>,
    pub color: Color,
    pub loaded: bool,
}

/// The comparison chart canvas state.
pub struct SpaghettiCanvas {
    pub series: Vec<Series>,
    pub cache: canvas::Cache,
    /// If true and at least two series are loaded, render A/B ratio.
    pub pair_ratio_mode: bool,
    /// In pair ratio mode, render as candlesticks when true.
    pub pair_candle_mode: bool,
    /// Monotonic token used to request viewport reset.
    pub reset_epoch: u64,
    /// Session-based normalization start time (ms). If None, uses the
    /// global earliest candle timestamp as a stable base.
    pub base_timestamp: Option<u64>,
    /// Which session is currently selected (for button highlighting).
    pub active_session: Option<Session>,
}

impl SpaghettiCanvas {
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            cache: canvas::Cache::new(),
            pair_ratio_mode: false,
            pair_candle_mode: false,
            reset_epoch: 0,
            base_timestamp: None,
            active_session: None,
        }
    }

    pub(super) fn loaded_series(&self) -> Vec<&Series> {
        self.series
            .iter()
            .filter(|s| s.loaded && !s.candles.is_empty())
            .collect()
    }

    /// Push or update a candle for a specific series, maintaining sorted order.
    pub fn push_candle(&mut self, symbol: &str, candle: Candle) {
        if !is_valid_candle(&candle) {
            return;
        }
        if let Some(s) = self.series.iter_mut().find(|s| s.symbol == symbol) {
            let ts = candle.open_time;
            // Check the common case: candle is the latest or updates the last
            if let Some(last) = s.candles.last_mut() {
                if last.open_time == ts {
                    *last = candle;
                } else if last.open_time < ts {
                    s.candles.push(candle);
                } else {
                    // Out-of-order: insert at correct sorted position
                    match s.candles.binary_search_by_key(&ts, |c| c.open_time) {
                        Ok(i) => s.candles[i] = candle,
                        Err(i) => s.candles.insert(i, candle),
                    }
                }
            } else {
                s.candles.push(candle);
            }
            self.cache.clear();
        }
    }
}
