mod helpers;
mod interaction;
mod model;
mod normalized;
mod ratio;
mod session;
mod state;
#[cfg(test)]
mod tests;

use crate::message::Message;
use iced::mouse;
use iced::widget::canvas;
use iced::{Rectangle, Renderer, Theme};

use self::helpers::{chart_time_window, global_time_range};
pub use self::model::ComparisonColorMode;
pub use self::model::{Series, SpaghettiCanvas, series_colors};
use self::normalized::NormalizedRenderContext;
use self::ratio::PairRatioRenderContext;
use self::state::SpaghettiChartState;
pub use session::{SESSION_OPTIONS, Session};

// ---------------------------------------------------------------------------
// Spaghetti (Comparison) Chart — Canvas-based
// ---------------------------------------------------------------------------

const PRICE_AXIS_WIDTH: f32 = 60.0;
const TIME_AXIS_HEIGHT: f32 = 24.0;
const PRICE_PADDING_PCT: f64 = 0.08;

const ZOOM_SPEED: f32 = 1.12;

// ---------------------------------------------------------------------------
// canvas::Program implementation
// ---------------------------------------------------------------------------

impl canvas::Program<Message> for SpaghettiCanvas {
    type State = SpaghettiChartState;

    fn update(
        &self,
        state: &mut SpaghettiChartState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        self.update_interaction(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &SpaghettiChartState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let loaded_series = self.loaded_series();
        if loaded_series.is_empty() {
            return vec![];
        }

        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let chart_h = bounds.height - TIME_AXIS_HEIGHT;
        if chart_w <= 0.0 || chart_h <= 0.0 {
            return vec![];
        }

        let Some((global_min_ts, global_max_ts)) = global_time_range(&loaded_series) else {
            return vec![];
        };

        let effective_max = if global_min_ts == global_max_ts {
            global_max_ts + 3_600_000
        } else {
            global_max_ts
        };

        let (left_ts, right_ts, visible_ms, time_px_per_ms) = chart_time_window(
            effective_max,
            self.base_timestamp,
            self.active_session,
            state.scroll_offset_ms,
            state.px_per_ms,
            chart_w,
        );

        if self.pair_ratio_mode && loaded_series.len() >= 2 {
            return self.draw_pair_ratio(
                PairRatioRenderContext {
                    state,
                    renderer,
                    theme,
                    bounds,
                    chart_w,
                    chart_h,
                    left_ts,
                    right_ts,
                    visible_ms,
                    time_px_per_ms,
                    effective_max,
                    base_timestamp: self.base_timestamp,
                },
                &loaded_series,
            );
        }

        self.draw_normalized(
            NormalizedRenderContext {
                state,
                renderer,
                theme,
                bounds,
                chart_w,
                chart_h,
                left_ts,
                right_ts,
                visible_ms,
                time_px_per_ms,
                effective_max,
                base_ts: self.base_timestamp.unwrap_or(global_min_ts),
            },
            &loaded_series,
        )
    }

    fn mouse_interaction(
        &self,
        state: &SpaghettiChartState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        self.mouse_interaction_for(state, bounds, cursor)
    }
}
