use super::CandlestickChart;
use crate::helpers::ease_out_cubic;

// ---------------------------------------------------------------------------
// Order Cancel Hover Animation
// ---------------------------------------------------------------------------

const ORDER_CANCEL_HOVER_EASE: f32 = 0.32;
const ORDER_CANCEL_HOVER_EPSILON: f32 = 0.01;

impl CandlestickChart {
    pub(crate) fn set_order_cancel_hover(&mut self, oid: Option<u64>) {
        if self.hover_order_cancel_oid == oid {
            return;
        }
        self.hover_order_cancel_oid = oid;
        if oid.is_some() {
            self.order_cancel_hover_progress = self.order_cancel_hover_progress.min(0.35);
        }
    }

    pub(crate) fn order_cancel_hover_animation_active(&self) -> bool {
        self.hover_order_cancel_oid.is_some() || self.order_cancel_hover_progress > 0.0
    }

    pub(crate) fn advance_order_cancel_hover_animation(&mut self) {
        let target = if self.hover_order_cancel_oid.is_some() {
            1.0
        } else {
            0.0
        };
        let delta = target - self.order_cancel_hover_progress;
        if delta.abs() <= ORDER_CANCEL_HOVER_EPSILON {
            self.order_cancel_hover_progress = target;
            return;
        }

        self.order_cancel_hover_progress =
            (self.order_cancel_hover_progress + delta * ORDER_CANCEL_HOVER_EASE).clamp(0.0, 1.0);
    }

    pub(in crate::chart) fn order_cancel_hover_progress(&self) -> f32 {
        self.order_cancel_hover_progress
    }

    pub(in crate::chart) fn order_cancel_hover_progress_for(&self, oid: u64) -> f32 {
        if self.hover_order_cancel_oid == Some(oid) {
            ease_out_cubic(self.order_cancel_hover_progress)
        } else {
            0.0
        }
    }
}
