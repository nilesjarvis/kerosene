use super::state::DEFAULT_PX_PER_MS;
use super::{Series, Session};
use crate::api::Candle;

pub(super) fn has_positive_finite_prices(candle: &Candle) -> bool {
    candle.open.is_finite()
        && candle.high.is_finite()
        && candle.low.is_finite()
        && candle.close.is_finite()
        && candle.open > 0.0
        && candle.high > 0.0
        && candle.low > 0.0
        && candle.close > 0.0
        && candle.low <= candle.high
        && candle.low <= candle.open
        && candle.low <= candle.close
        && candle.high >= candle.open
        && candle.high >= candle.close
}

/// Find the candle closest to a given timestamp using binary search.
pub(super) fn find_candle_at(candles: &[Candle], ts: u64) -> Option<usize> {
    if candles.is_empty() {
        return None;
    }
    match candles.binary_search_by_key(&ts, |c| c.open_time) {
        Ok(i) => Some(i),
        Err(i) => {
            if i == 0 {
                Some(0)
            } else if i >= candles.len() {
                Some(candles.len() - 1)
            } else {
                let before = ts.saturating_sub(candles[i - 1].open_time);
                let after = candles[i].open_time.saturating_sub(ts);
                if before <= after {
                    Some(i - 1)
                } else {
                    Some(i)
                }
            }
        }
    }
}

/// Get the global timestamp range across all loaded series.
pub(super) fn global_time_range(series: &[&Series]) -> Option<(u64, u64)> {
    let mut min_ts = u64::MAX;
    let mut max_ts = 0u64;
    for s in series {
        if let Some(first) = s.candles.first() {
            min_ts = min_ts.min(first.open_time);
        }
        if let Some(last) = s.candles.last() {
            max_ts = max_ts.max(last.open_time);
        }
    }
    if min_ts <= max_ts && max_ts > 0 {
        Some((min_ts, max_ts))
    } else {
        None
    }
}

/// Format a relative time label (e.g., "now", "1h ago", "2h ago").
pub(super) fn format_relative_time(delta_ms: f64) -> String {
    let secs = delta_ms / 1000.0;
    if secs < 60.0 {
        "now".to_string()
    } else if secs < 3600.0 {
        format!("{:.0}m ago", secs / 60.0)
    } else if secs < 86400.0 {
        let hours = secs / 3600.0;
        if hours < 10.0 {
            format!("{:.1}h ago", hours)
        } else {
            format!("{:.0}h ago", hours)
        }
    } else {
        format!("{:.1}d ago", secs / 86400.0)
    }
}

pub(super) fn chart_time_window(
    effective_max: u64,
    base_timestamp: Option<u64>,
    active_session: Option<Session>,
    scroll_offset_ms: f64,
    px_per_ms: f64,
    chart_w: f32,
) -> (f64, f64, f64, f64) {
    if let Some(base_ts) = base_timestamp
        && active_session.is_some()
    {
        let time_px_per_ms = anchored_time_px_per_ms(effective_max, base_ts, px_per_ms, chart_w);
        let span = (chart_w as f64 / time_px_per_ms).max(1.0);
        let max_scroll = anchored_max_scroll_offset(effective_max, base_ts, px_per_ms, chart_w);
        let left = base_ts as f64 + scroll_offset_ms.clamp(0.0, max_scroll);
        return (left, left + span, span, time_px_per_ms);
    }

    let right = effective_max as f64 - scroll_offset_ms;
    let span = (chart_w as f64 / px_per_ms).max(1.0);
    (right - span, right, span, px_per_ms)
}

pub(super) fn anchored_time_px_per_ms(
    effective_max: u64,
    base_timestamp: u64,
    px_per_ms: f64,
    chart_w: f32,
) -> f64 {
    let data_span = anchored_data_span(effective_max, base_timestamp);
    let fit_px_per_ms = (chart_w as f64 / data_span).max(f64::EPSILON);
    let zoom_scale = (px_per_ms / DEFAULT_PX_PER_MS).max(f64::EPSILON);
    fit_px_per_ms * zoom_scale
}

pub(super) fn anchored_max_scroll_offset(
    effective_max: u64,
    base_timestamp: u64,
    px_per_ms: f64,
    chart_w: f32,
) -> f64 {
    let data_span = anchored_data_span(effective_max, base_timestamp);
    let time_px_per_ms = anchored_time_px_per_ms(effective_max, base_timestamp, px_per_ms, chart_w);
    let visible_span = (chart_w as f64 / time_px_per_ms).max(1.0);
    (data_span - visible_span).max(0.0)
}

fn anchored_data_span(effective_max: u64, base_timestamp: u64) -> f64 {
    (effective_max as f64 - base_timestamp as f64).max(60_000.0)
}
