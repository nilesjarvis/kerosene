use super::CandlestickChart;
use crate::helpers::ease_out_cubic;

// ---------------------------------------------------------------------------
// Earnings Marker Hover Animation
// ---------------------------------------------------------------------------

const EARNINGS_MARKER_HOVER_EASE: f32 = 0.32;
const EARNINGS_MARKER_HOVER_EPSILON: f32 = 0.01;

impl CandlestickChart {
    pub(crate) fn set_earnings_marker_hover(&mut self, time_ms: Option<u64>) {
        if self.hover_earnings_marker_time_ms == time_ms {
            return;
        }
        self.hover_earnings_marker_time_ms = time_ms;
        if time_ms.is_some() {
            self.earnings_marker_hover_progress = self.earnings_marker_hover_progress.min(0.35);
        }
    }

    pub(crate) fn earnings_marker_hover_animation_active(&self) -> bool {
        self.hover_earnings_marker_time_ms.is_some() || self.earnings_marker_hover_progress > 0.0
    }

    pub(crate) fn advance_earnings_marker_hover_animation(&mut self) {
        let target = if self.hover_earnings_marker_time_ms.is_some() {
            1.0
        } else {
            0.0
        };
        let delta = target - self.earnings_marker_hover_progress;
        if delta.abs() <= EARNINGS_MARKER_HOVER_EPSILON {
            self.earnings_marker_hover_progress = target;
            return;
        }

        self.earnings_marker_hover_progress = (self.earnings_marker_hover_progress
            + delta * EARNINGS_MARKER_HOVER_EASE)
            .clamp(0.0, 1.0);
    }

    pub(in crate::chart) fn earnings_marker_hover_progress_for(&self, time_ms: u64) -> f32 {
        if self.hover_earnings_marker_time_ms == Some(time_ms) {
            ease_out_cubic(self.earnings_marker_hover_progress)
        } else {
            0.0
        }
    }
}
