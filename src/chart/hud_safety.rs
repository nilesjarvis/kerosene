use super::CandlestickChart;
use crate::config::ChartCrosshairStyle;

// ---------------------------------------------------------------------------
// HUD Safety Arm
// ---------------------------------------------------------------------------

pub(crate) const HUD_ARM_IDLE_TIMEOUT_MS: u64 = 15_000;

impl CandlestickChart {
    pub(crate) fn hud_armed(&self) -> bool {
        self.hud_armed
    }

    pub(crate) fn hud_order_submission_enabled(&self) -> bool {
        self.hud_armed && self.crosshair_style.normalized() == ChartCrosshairStyle::Hud
    }

    pub(crate) fn toggle_hud_armed_at(&mut self, now_ms: u64) {
        self.set_hud_armed_at(!self.hud_armed, now_ms);
    }

    pub(crate) fn set_hud_armed_at(&mut self, armed: bool, now_ms: u64) {
        if self.hud_armed == armed {
            if armed {
                self.record_hud_activity(now_ms, true);
            }
            return;
        }

        self.hud_armed = armed;
        if armed {
            self.hud_last_activity_ms = Some(now_ms);
            self.hud_hovering = true;
        } else {
            self.hud_last_activity_ms = None;
            self.hud_hovering = false;
        }
    }

    pub(crate) fn clear_hud_armed(&mut self) {
        self.hud_armed = false;
        self.hud_last_activity_ms = None;
        self.hud_hovering = false;
    }

    pub(crate) fn record_hud_activity(&mut self, now_ms: u64, hovering: bool) {
        self.hud_hovering = hovering;
        if self.hud_armed {
            self.hud_last_activity_ms = Some(now_ms);
        }
    }

    pub(crate) fn hud_safety_timeout_due(&self, now_ms: u64) -> bool {
        if !self.hud_armed
            || self.hud_hovering
            || self.crosshair_style.normalized() != ChartCrosshairStyle::Hud
        {
            return false;
        }

        self.hud_last_activity_ms
            .map(|last_activity_ms| {
                now_ms.saturating_sub(last_activity_ms) >= HUD_ARM_IDLE_TIMEOUT_MS
            })
            .unwrap_or(true)
    }
}
