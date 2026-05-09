use super::super::state::{MAX_PX_PER_MS, MIN_PX_PER_MS};

#[cfg(test)]
mod tests;

pub(super) fn ratio_zoom_speed(ratio: f64) -> f64 {
    if ratio <= 0.01 {
        1.03
    } else if ratio <= 0.05 {
        1.05
    } else if ratio <= 0.20 {
        1.07
    } else if ratio <= 5.0 {
        1.12
    } else if ratio <= 50.0 {
        1.16
    } else if ratio <= 500.0 {
        1.20
    } else if ratio <= 2_000.0 {
        1.24
    } else {
        1.28
    }
}

pub(super) fn zoomed_px_per_ms(current_px_per_ms: f64, factor: f64) -> f64 {
    (current_px_per_ms * factor).clamp(MIN_PX_PER_MS, MAX_PX_PER_MS)
}

pub(super) fn scroll_offset_for_zoom(
    old_px_per_ms: f64,
    new_px_per_ms: f64,
    current_scroll_offset_ms: f64,
    chart_w: f32,
    cursor_x: f32,
) -> f64 {
    let px_from_right = f64::from(chart_w - cursor_x);
    let ms_from_right = px_from_right / old_px_per_ms + current_scroll_offset_ms;
    ms_from_right - px_from_right / new_px_per_ms
}

pub(super) fn anchored_scroll_offset_for_zoom(
    old_px_per_ms: f64,
    new_px_per_ms: f64,
    current_scroll_offset_ms: f64,
    cursor_x: f32,
) -> f64 {
    let ms_from_left = f64::from(cursor_x) / old_px_per_ms + current_scroll_offset_ms;
    ms_from_left - f64::from(cursor_x) / new_px_per_ms
}

pub(super) fn minimum_scroll_offset(
    chart_w: f32,
    px_per_ms: f64,
    pair_ratio_mode: bool,
    has_active_session: bool,
) -> f64 {
    if pair_ratio_mode && !has_active_session {
        -(f64::from(chart_w) / px_per_ms) * 0.75
    } else {
        0.0
    }
}
