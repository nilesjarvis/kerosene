use crate::chart::model::VisibleCandleRange;
use crate::chart::price_range::visible_price_stats;
use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartState, VOLUME_REGION_RATIO};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Visible Range And Price Parameters
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart) fn visible_candle_range(
        &self,
        state: &ChartState,
        chart_w: f32,
    ) -> Option<VisibleCandleRange> {
        if self.candles.is_empty()
            || chart_w <= 0.0
            || !chart_w.is_finite()
            || state.candle_width <= 0.0
            || !state.candle_width.is_finite()
        {
            return None;
        }

        let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
        let last_idx = self.candles.len() as isize - 1;
        let visible_slots = (chart_w / step).ceil() as isize + 1;
        let scroll_offset = self.effective_scroll_offset(state, chart_w);
        let right_idx = last_idx - scroll_offset as isize;
        let left_idx = right_idx - visible_slots;

        let first_vis = (left_idx + 1).clamp(0, last_idx) as usize;
        let last_vis = right_idx.clamp(0, last_idx) as usize;
        if first_vis <= last_vis {
            Some(VisibleCandleRange {
                first: first_vis,
                last: last_vis,
                right_idx,
            })
        } else {
            let fallback = right_idx.clamp(0, last_idx) as usize;
            Some(VisibleCandleRange {
                first: fallback,
                last: fallback,
                right_idx: fallback as isize,
            })
        }
    }

    /// Compute the visible price parameters (price_hi, price_range, price_h)
    /// needed for Y-coordinate <-> price conversions in the `update()` method.
    /// Returns `None` if there are no candles or the range is zero.
    pub(in crate::chart) fn visible_price_params(
        &self,
        state: &ChartState,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<(f64, f64, f32)> {
        if self.candles.is_empty()
            || chart_w <= 0.0
            || chart_h <= 0.0
            || !chart_w.is_finite()
            || !chart_h.is_finite()
        {
            return None;
        }
        let price_h = chart_h * (1.0 - VOLUME_REGION_RATIO);
        if price_h <= 0.0 || !price_h.is_finite() {
            return None;
        }

        let visible_range = self.visible_candle_range(state, chart_w)?;
        let first_vis = visible_range.first;
        let last_vis = visible_range.last;

        let price_stats = visible_price_stats(
            &self.candles[first_vis..=last_vis],
            state.y_auto,
            state.y_scale,
            state.y_offset,
        )?;
        let price_range = price_stats.price_range;
        if price_range <= 0.0 {
            return None;
        }
        Some((price_stats.price_hi, price_range, price_h))
    }

    /// Compute the visible price range for the current viewport.
    pub(in crate::chart) fn visible_price_range(&self, state: &ChartState, chart_w: f32) -> f64 {
        if self.candles.is_empty() {
            return 1.0;
        }
        let Some(visible_range) = self.visible_candle_range(state, chart_w) else {
            return 1.0;
        };
        let first_vis = visible_range.first;
        let last_vis = visible_range.last;

        let Some(price_stats) = visible_price_stats(
            &self.candles[first_vis..=last_vis],
            state.y_auto,
            state.y_scale,
            state.y_offset,
        ) else {
            return 1.0;
        };
        if price_stats.price_range > 0.0 {
            price_stats.price_range
        } else {
            1.0
        }
    }

    pub(in crate::chart) fn price_to_y_with(
        &self,
        price: f64,
        price_hi: f64,
        price_range: f64,
        price_h: f32,
    ) -> f32 {
        if self.inverted {
            let price_lo = price_hi - price_range;
            ((price - price_lo) / price_range * price_h as f64) as f32
        } else {
            ((price_hi - price) / price_range * price_h as f64) as f32
        }
    }

    pub(in crate::chart) fn y_to_price_with(
        &self,
        y: f32,
        price_hi: f64,
        price_range: f64,
        price_h: f32,
    ) -> f64 {
        if self.inverted {
            let price_lo = price_hi - price_range;
            price_lo + (y as f64 / price_h as f64) * price_range
        } else {
            price_hi - (y as f64 / price_h as f64) * price_range
        }
    }
}
