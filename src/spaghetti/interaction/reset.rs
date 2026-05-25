use super::super::helpers::has_positive_finite_prices;
use super::super::state::{DEFAULT_PX_PER_MS, MAX_PX_PER_MS, MIN_PX_PER_MS};
use super::super::{Series, SpaghettiCanvas};

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Reset Zoom
// ---------------------------------------------------------------------------

impl SpaghettiCanvas {
    pub(super) fn reset_px_per_ms(&self, chart_w: f32, loaded: &[&Series]) -> f64 {
        if self.pair_ratio_mode && self.active_session.is_none() {
            pair_ratio_reset_px_per_ms(chart_w, loaded).unwrap_or(DEFAULT_PX_PER_MS)
        } else {
            DEFAULT_PX_PER_MS
        }
    }
}

const PAIR_RATIO_RESET_TARGET_CANDLES: usize = 96;

fn pair_ratio_reset_px_per_ms(chart_w: f32, loaded: &[&Series]) -> Option<f64> {
    if chart_w <= 0.0 || loaded.len() < 2 {
        return None;
    }

    let b_times: HashSet<u64> = loaded[1]
        .candles
        .iter()
        .filter(|candle| has_positive_finite_prices(candle))
        .map(|candle| candle.open_time)
        .collect();
    let overlap: Vec<u64> = loaded[0]
        .candles
        .iter()
        .filter(|candle| has_positive_finite_prices(candle) && b_times.contains(&candle.open_time))
        .map(|candle| candle.open_time)
        .collect();

    if overlap.len() < 2 {
        return None;
    }

    let first_idx = overlap
        .len()
        .saturating_sub(PAIR_RATIO_RESET_TARGET_CANDLES);
    let first_visible = overlap[first_idx];
    let last_visible = *overlap.last()?;
    let target_span = last_visible.saturating_sub(first_visible);
    if target_span == 0 {
        return None;
    }

    let default_visible_ms = f64::from(chart_w) / DEFAULT_PX_PER_MS;
    let span_ms = (target_span as f64).max(default_visible_ms);
    Some((f64::from(chart_w) / span_ms).clamp(MIN_PX_PER_MS, MAX_PX_PER_MS))
}

#[cfg(test)]
mod tests;
