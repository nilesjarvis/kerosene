use super::{CandlestickChart, ChartStatus};
use crate::api::{Candle, is_valid_candle, normalize_candles};
use iced::Color;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Chart Data Lifecycle
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            candles: Vec::new(),
            status: ChartStatus::Loading,
            candle_cache: canvas::Cache::new(),
            reset_epoch: 0,
            active_position: None,
            active_orders: Vec::new(),
            annotations: Vec::new(),
            active_tool: None,
            liquidation_buckets: Vec::new(),
            heatmap_rects: Vec::new(),
            heatmap_max_usd: 0.0,
            macro_indicators: crate::config::MacroIndicatorsConfig::default(),
            daily_candles: Vec::new(),
            weekly_candles: Vec::new(),
            monthly_candles: Vec::new(),
            inverted: false,
            chart_bull_color: None,
            chart_bear_color: None,
        }
    }

    pub fn request_view_reset(&mut self) {
        self.reset_epoch = self.reset_epoch.saturating_add(1);
        self.candle_cache.clear();
    }

    pub fn set_chart_colors(&mut self, bull: Option<Color>, bear: Option<Color>) {
        if self.chart_bull_color != bull || self.chart_bear_color != bear {
            self.chart_bull_color = bull;
            self.chart_bear_color = bear;
            self.candle_cache.clear();
        }
    }

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
}
