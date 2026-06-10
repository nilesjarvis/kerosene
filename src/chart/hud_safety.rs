use super::CandlestickChart;
// ---------------------------------------------------------------------------
// HUD Safety Arm
// ---------------------------------------------------------------------------

pub(crate) const HUD_ARM_IDLE_TIMEOUT_MS: u64 = 15_000;
/// Advisory pip plays once when this little idle time remains before auto-disarm.
pub(crate) const HUD_IDLE_WARNING_REMAINING_MS: u64 = 3_000;

impl CandlestickChart {
    pub(crate) fn hud_armed(&self) -> bool {
        self.hud_armed
    }

    pub(crate) fn hud_order_submission_enabled(&self) -> bool {
        self.hud_armed && self.crosshair_style.is_game_hud()
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
        self.hud_idle_warning_sounded = false;
        // Restart the pulse so every arm session ramps from the same phase;
        // the animation tick stops on disarm, so reset here, not there.
        self.hud_pulse_phase = 0.0;
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
        self.hud_idle_warning_sounded = false;
        self.hud_pulse_phase = 0.0;
    }

    pub(crate) fn record_hud_activity(&mut self, now_ms: u64, hovering: bool) {
        self.hud_hovering = hovering;
        if self.hud_armed {
            self.hud_last_activity_ms = Some(now_ms);
            self.hud_idle_warning_sounded = false;
        }
    }

    pub(crate) fn hud_safety_timeout_due(&self, now_ms: u64) -> bool {
        if !self.hud_armed || self.hud_hovering || !self.crosshair_style.is_game_hud() {
            return false;
        }

        self.hud_idle_elapsed_ms(now_ms)
            .map(|elapsed_ms| elapsed_ms >= HUD_ARM_IDLE_TIMEOUT_MS)
            .unwrap_or(true)
    }

    /// True once per arm session when the idle fuse is about to auto-disarm.
    pub(crate) fn hud_safety_warning_due(&self, now_ms: u64) -> bool {
        if self.hud_idle_warning_sounded
            || !self.hud_armed
            || self.hud_hovering
            || !self.crosshair_style.is_game_hud()
        {
            return false;
        }

        self.hud_idle_elapsed_ms(now_ms).is_some_and(|elapsed_ms| {
            elapsed_ms < HUD_ARM_IDLE_TIMEOUT_MS
                && HUD_ARM_IDLE_TIMEOUT_MS - elapsed_ms <= HUD_IDLE_WARNING_REMAINING_MS
        })
    }

    pub(crate) fn mark_hud_idle_warning_sounded(&mut self) {
        self.hud_idle_warning_sounded = true;
    }

    /// Click-time HUD limit side inference: at or below the market reference
    /// price rests a bid (buy), above rests an ask (sell). The single source
    /// of truth for both the press handler that fires the order and the HUD
    /// order summary that previews it.
    pub(in crate::chart) fn hud_limit_click_is_buy(&self, price: f64) -> Option<bool> {
        self.market_reference_price
            .or_else(|| self.candles.last().map(|candle| candle.close))
            .map(|reference| price <= reference)
    }

    fn hud_idle_elapsed_ms(&self, now_ms: u64) -> Option<u64> {
        self.hud_last_activity_ms
            .map(|last_activity_ms| now_ms.saturating_sub(last_activity_ms))
    }
}
