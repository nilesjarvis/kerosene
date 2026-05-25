use super::ChartInstance;

// ---------------------------------------------------------------------------
// Price Flash State
// ---------------------------------------------------------------------------

pub(crate) const CHART_PRICE_FLASH_MS: u64 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriceFlashDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PriceFlash {
    pub(crate) started_at_ms: u64,
    pub(crate) direction: PriceFlashDirection,
    pub(crate) previous_close: f64,
}

impl ChartInstance {
    pub(crate) fn track_last_price_update(
        &mut self,
        previous_close: Option<f64>,
        next_close: f64,
        now_ms: u64,
    ) {
        let Some(previous_close) = previous_close else {
            return;
        };
        if !previous_close.is_finite() || !next_close.is_finite() {
            return;
        }
        if (next_close - previous_close).abs() <= f64::EPSILON {
            return;
        }
        let direction = if next_close > previous_close {
            PriceFlashDirection::Up
        } else {
            PriceFlashDirection::Down
        };
        self.last_price_flash = Some(PriceFlash {
            started_at_ms: now_ms,
            direction,
            previous_close,
        });
    }

    pub(crate) fn last_price_flash_is_active(&self, now_ms: u64) -> bool {
        self.last_price_flash
            .is_some_and(|flash| now_ms.saturating_sub(flash.started_at_ms) < CHART_PRICE_FLASH_MS)
    }

    pub(crate) fn clear_expired_last_price_flash(&mut self, now_ms: u64) {
        if self.last_price_flash_is_active(now_ms) {
            return;
        }
        self.last_price_flash = None;
    }
}
