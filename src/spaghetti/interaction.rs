use super::helpers::{anchored_max_scroll_offset, anchored_time_px_per_ms, global_time_range};
use super::state::{DEFAULT_PX_PER_MS, DragKind, MAX_PX_PER_MS, MIN_PX_PER_MS};
use super::{PRICE_AXIS_WIDTH, SpaghettiCanvas, SpaghettiChartState, TIME_AXIS_HEIGHT};
use crate::api::Candle;
use crate::message::Message;
use iced::Rectangle;
use iced::mouse;
use iced::widget::canvas;
use std::collections::HashSet;

mod drag;
mod rules;
mod wheel;

use rules::minimum_scroll_offset;

// ---------------------------------------------------------------------------
// Canvas Interaction
// ---------------------------------------------------------------------------

impl SpaghettiCanvas {
    pub(super) fn update_interaction(
        &self,
        state: &mut SpaghettiChartState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.position_in(bounds);
        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let chart_h = bounds.height - TIME_AXIS_HEIGHT;
        let needs_redraw = state.cursor_position != pos;
        state.cursor_position = pos;

        let loaded = self.loaded_series();

        if state.reset_epoch_seen != self.reset_epoch {
            let reset_px_per_ms = self.reset_px_per_ms(chart_w, &loaded);
            state.reset_view_with_px(self.reset_epoch, reset_px_per_ms);
            self.cache.clear();
            return Some(canvas::Action::request_redraw());
        }

        let pair_latest_ratio = self.pair_latest_ratio_for(&loaded);
        let (effective_max, unanchored_max_scroll) =
            if let Some((min, max)) = global_time_range(&loaded) {
                let effective_max = if min == max { max + 3_600_000 } else { max };
                (effective_max, (max - min) as f64)
            } else {
                (0, 0.0)
            };
        let max_scroll = self.max_scroll_for(
            chart_w,
            state.px_per_ms,
            unanchored_max_scroll,
            effective_max,
        );
        let horizontal_px_per_ms =
            self.horizontal_time_px_per_ms(chart_w, state.px_per_ms, effective_max);

        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => self.handle_wheel_scroll(
                state,
                delta,
                pos?,
                wheel::WheelScrollContext {
                    chart_w,
                    chart_h,
                    unanchored_max_scroll,
                    effective_max,
                    pair_latest_ratio,
                },
            ),
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                self.begin_left_drag(state, pos?, chart_w, chart_h);
                None
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(action) = self.handle_drag_move(
                    state,
                    pos,
                    chart_w,
                    chart_h,
                    horizontal_px_per_ms,
                    max_scroll,
                ) {
                    return Some(action);
                }
                if needs_redraw {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.finish_left_drag(state)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                self.reset_y_axis_at(state, pos?, chart_w, chart_h)
            }
            _ => {
                if needs_redraw {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn mouse_interaction_for(
        &self,
        state: &SpaghettiChartState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(pos) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let chart_h = bounds.height - TIME_AXIS_HEIGHT;
        match state.drag {
            Some(DragKind::PanX) => mouse::Interaction::Grabbing,
            Some(DragKind::PanY) => mouse::Interaction::ResizingVertically,
            None => {
                if pos.x >= chart_w && pos.y < chart_h {
                    mouse::Interaction::ResizingVertically
                } else if pos.x < chart_w && pos.y < chart_h {
                    mouse::Interaction::Crosshair
                } else {
                    mouse::Interaction::default()
                }
            }
        }
    }

    fn min_scroll_for(&self, chart_w: f32, px_per_ms: f64) -> f64 {
        minimum_scroll_offset(
            chart_w,
            px_per_ms,
            self.pair_ratio_mode,
            self.active_session.is_some(),
        )
    }

    pub(super) fn max_scroll_for(
        &self,
        chart_w: f32,
        px_per_ms: f64,
        unanchored_max_scroll: f64,
        effective_max: u64,
    ) -> f64 {
        if let Some(base_ts) = self.base_timestamp
            && self.active_session.is_some()
        {
            return anchored_max_scroll_offset(effective_max, base_ts, px_per_ms, chart_w);
        }

        unanchored_max_scroll
    }

    pub(super) fn horizontal_time_px_per_ms(
        &self,
        chart_w: f32,
        px_per_ms: f64,
        effective_max: u64,
    ) -> f64 {
        if let Some(base_ts) = self.base_timestamp
            && self.active_session.is_some()
        {
            return anchored_time_px_per_ms(effective_max, base_ts, px_per_ms, chart_w);
        }

        px_per_ms
    }

    fn reset_px_per_ms(&self, chart_w: f32, loaded: &[&super::Series]) -> f64 {
        if self.pair_ratio_mode && self.active_session.is_none() {
            pair_ratio_reset_px_per_ms(chart_w, loaded).unwrap_or(DEFAULT_PX_PER_MS)
        } else {
            DEFAULT_PX_PER_MS
        }
    }
}

const PAIR_RATIO_RESET_TARGET_CANDLES: usize = 96;

fn pair_ratio_reset_px_per_ms(chart_w: f32, loaded: &[&super::Series]) -> Option<f64> {
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

fn has_positive_finite_prices(candle: &Candle) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spaghetti::Series;
    use iced::Color;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn candle_at(open_time: u64, close: f64) -> Candle {
        Candle {
            open_time,
            close_time: open_time + 59_999,
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1.0,
        }
    }

    fn series(symbol: &str, candles: Vec<Candle>) -> Series {
        Series {
            symbol: symbol.to_string(),
            display: symbol.to_string(),
            candles,
            color: Color::WHITE,
            loaded: true,
        }
    }

    #[test]
    fn pair_ratio_reset_keeps_intraday_default_window() {
        let chart_w = 720.0;
        let mut a_candles = Vec::new();
        let mut b_candles = Vec::new();
        for idx in 0..48 {
            let ts = idx * 3_600_000;
            a_candles.push(candle_at(ts, 10.0));
            b_candles.push(candle_at(ts, 20.0));
        }
        let a = series("HYPE", a_candles);
        let b = series("BTC", b_candles);

        let px = pair_ratio_reset_px_per_ms(chart_w, &[&a, &b]).expect("reset px");

        assert_close(px, DEFAULT_PX_PER_MS);
    }

    #[test]
    fn pair_ratio_reset_fits_high_timeframe_overlap() {
        let chart_w = 720.0;
        let day_ms = 86_400_000;
        let mut a_candles = Vec::new();
        let mut b_candles = Vec::new();
        for idx in 0..120 {
            let ts = idx * day_ms;
            a_candles.push(candle_at(ts, 10.0));
            b_candles.push(candle_at(ts, 20.0));
        }
        let a = series("HYPE", a_candles);
        let b = series("BTC", b_candles);

        let px = pair_ratio_reset_px_per_ms(chart_w, &[&a, &b]).expect("reset px");
        let visible_days = chart_w as f64 / px / day_ms as f64;

        assert!(
            visible_days >= 94.0,
            "expected at least 94 visible days, got {visible_days}"
        );
        assert!(
            visible_days <= 96.0,
            "expected at most 96 visible days, got {visible_days}"
        );
    }
}
