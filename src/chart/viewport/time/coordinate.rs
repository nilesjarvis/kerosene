use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartState};

// ---------------------------------------------------------------------------
// Timestamp Coordinate Mapping
// ---------------------------------------------------------------------------

impl CandlestickChart {
    /// Convert a pixel X coordinate to an interpolated timestamp.
    /// Uses fractional indices between candles for smooth sub-candle precision.
    pub(in crate::chart) fn x_to_timestamp(
        &self,
        x: f32,
        state: &ChartState,
        chart_w: f32,
    ) -> Option<u64> {
        if self.candles.is_empty() {
            return None;
        }
        if chart_w <= 0.0 || state.candle_width <= 0.0 {
            return None;
        }
        let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
        let last_idx = self.candles.len() as isize - 1;
        let scroll_offset = self.effective_scroll_offset(state, chart_w);
        let right_idx = last_idx as f64 - scroll_offset as f64;
        // Shift by 0.5 step so that the exact center of the candle represents its exact timestamp.
        let slots_from_right = ((chart_w - x) / step) as f64 - 0.5;
        let float_idx = right_idx - slots_from_right;

        let floor_idx = float_idx.floor() as isize;
        let ceil_idx = float_idx.ceil() as isize;

        if floor_idx < 0 {
            let t0 = self.candles[0].open_time as f64;
            let t1 = self
                .candles
                .get(1)
                .map(|c| c.open_time as f64)
                .unwrap_or(t0 + 60000.0);
            let dt = t1 - t0;
            let ts = t0 + float_idx * dt;
            return Some(ts as u64);
        }
        if ceil_idx > last_idx {
            let t1 = self.candles[last_idx as usize].open_time as f64;
            let t0 = if last_idx > 0 {
                self.candles[last_idx as usize - 1].open_time as f64
            } else {
                t1 - 60000.0
            };
            let dt = t1 - t0;
            let ts = t1 + (float_idx - last_idx as f64) * dt;
            return Some(ts as u64);
        }

        if floor_idx == ceil_idx {
            return Some(self.candles[floor_idx as usize].open_time);
        }

        let t0 = self.candles[floor_idx as usize].open_time as f64;
        let t1 = self.candles[ceil_idx as usize].open_time as f64;
        let fraction = float_idx - floor_idx as f64;

        let ts = t0 + (t1 - t0) * fraction;
        Some(ts as u64)
    }

    /// Convert a timestamp to a pixel X coordinate.
    /// Uses binary search to find the bounding candles and interpolates the exact X.
    pub(in crate::chart) fn timestamp_to_x(
        &self,
        ts: u64,
        state: &ChartState,
        chart_w: f32,
    ) -> Option<f32> {
        if self.candles.is_empty() {
            return None;
        }
        if chart_w <= 0.0 || state.candle_width <= 0.0 {
            return None;
        }

        let float_idx = match self.candles.binary_search_by_key(&ts, |c| c.open_time) {
            Ok(i) => i as f64,
            Err(i) => {
                if i == 0 {
                    let t0 = self.candles[0].open_time as f64;
                    let t1 = self
                        .candles
                        .get(1)
                        .map(|c| c.open_time as f64)
                        .unwrap_or(t0 + 60000.0);
                    let dt = t1 - t0;
                    (ts as f64 - t0) / dt
                } else if i >= self.candles.len() {
                    let last_idx = self.candles.len() - 1;
                    let t1 = self.candles[last_idx].open_time as f64;
                    let t0 = if last_idx > 0 {
                        self.candles[last_idx - 1].open_time as f64
                    } else {
                        t1 - 60000.0
                    };
                    let dt = t1 - t0;
                    last_idx as f64 + (ts as f64 - t1) / dt
                } else {
                    let t0 = self.candles[i - 1].open_time as f64;
                    let t1 = self.candles[i].open_time as f64;
                    let dt = t1 - t0;
                    if dt <= 0.0 {
                        i as f64
                    } else {
                        let fraction = (ts as f64 - t0) / dt;
                        (i - 1) as f64 + fraction
                    }
                }
            }
        };

        let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
        let last_idx = self.candles.len() as f64 - 1.0;
        let scroll_offset = self.effective_scroll_offset(state, chart_w);
        let right_idx = last_idx - scroll_offset as f64;
        let slots_from_right = right_idx - float_idx;

        // Ensure the exact center of the candle hits the exact timestamp.
        let cx = chart_w as f64 - slots_from_right * (step as f64) - (step as f64) * 0.5;
        Some(cx as f32)
    }
}
