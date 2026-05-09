use super::super::{Series, SpaghettiCanvas, SpaghettiChartState, ZOOM_SPEED};
use super::rules::{
    anchored_scroll_offset_for_zoom, ratio_zoom_speed, scroll_offset_for_zoom, zoomed_px_per_ms,
};
use crate::message::Message;

use iced::Point;
use iced::mouse;
use iced::widget::canvas;

pub(super) struct WheelScrollContext {
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) unanchored_max_scroll: f64,
    pub(super) effective_max: u64,
    pub(super) pair_latest_ratio: Option<f64>,
}

impl SpaghettiCanvas {
    pub(super) fn pair_latest_ratio_for(&self, loaded: &[&Series]) -> Option<f64> {
        if !self.pair_ratio_mode || loaded.len() < 2 {
            return None;
        }

        let a = loaded[0].candles.last().map(|candle| candle.close);
        let b = loaded[1].candles.last().map(|candle| candle.close);
        match (a, b) {
            (Some(ac), Some(bc)) if ac > 0.0 && bc > 0.0 => Some(ac / bc),
            _ => None,
        }
    }

    pub(super) fn handle_wheel_scroll(
        &self,
        state: &mut SpaghettiChartState,
        delta: &mouse::ScrollDelta,
        pos: Point,
        ctx: WheelScrollContext,
    ) -> Option<canvas::Action<Message>> {
        let dy = match delta {
            mouse::ScrollDelta::Lines { y, .. } => *y,
            mouse::ScrollDelta::Pixels { y, .. } => *y / 28.0,
        };
        if dy == 0.0 {
            return None;
        }

        let zoom_speed = ctx
            .pair_latest_ratio
            .map(ratio_zoom_speed)
            .unwrap_or(ZOOM_SPEED as f64);
        if pos.x >= ctx.chart_w && pos.y < ctx.chart_h {
            zoom_y_axis(state, dy, zoom_speed);
        } else if pos.x < ctx.chart_w {
            self.zoom_time_axis(state, dy, zoom_speed, pos.x, &ctx);
        }

        self.cache.clear();
        Some(canvas::Action::request_redraw())
    }

    fn zoom_time_axis(
        &self,
        state: &mut SpaghettiChartState,
        dy: f32,
        zoom_speed: f64,
        cursor_x: f32,
        ctx: &WheelScrollContext,
    ) {
        let factor = if dy > 0.0 {
            zoom_speed
        } else {
            1.0 / zoom_speed
        };
        let old_px = state.px_per_ms;
        let new_px = zoomed_px_per_ms(old_px, factor);
        let new_offset = if self.active_session.is_some() && self.base_timestamp.is_some() {
            let old_time_px =
                self.horizontal_time_px_per_ms(ctx.chart_w, old_px, ctx.effective_max);
            let new_time_px =
                self.horizontal_time_px_per_ms(ctx.chart_w, new_px, ctx.effective_max);
            anchored_scroll_offset_for_zoom(
                old_time_px,
                new_time_px,
                state.scroll_offset_ms,
                cursor_x,
            )
        } else {
            scroll_offset_for_zoom(
                old_px,
                new_px,
                state.scroll_offset_ms,
                ctx.chart_w,
                cursor_x,
            )
        };
        state.px_per_ms = new_px;
        let max_scroll = self.max_scroll_for(
            ctx.chart_w,
            new_px,
            ctx.unanchored_max_scroll,
            ctx.effective_max,
        );
        state.scroll_offset_ms =
            new_offset.clamp(self.min_scroll_for(ctx.chart_w, new_px), max_scroll);
    }
}

fn zoom_y_axis(state: &mut SpaghettiChartState, dy: f32, zoom_speed: f64) {
    let factor = if dy > 0.0 {
        1.0 / zoom_speed
    } else {
        zoom_speed
    };
    state.y_scale = (state.y_scale * factor).clamp(0.1, 20.0);
    state.y_auto = false;
}
