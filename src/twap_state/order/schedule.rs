use super::super::model::{TwapEventKind, TwapOrder, TwapPauseReason, TwapStatus};
use super::super::{
    TWAP_MIN_INTERVAL, TWAP_RANDOM_JITTER, TWAP_RETRY_BASE_DELAY, TWAP_RETRY_MAX_DELAY,
};
use crate::helpers::{finite_value, positive_finite_value};

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// TWAP Scheduling
// ---------------------------------------------------------------------------

impl TwapOrder {
    pub(crate) fn can_schedule(&self) -> bool {
        !self.status.is_terminal() && !self.stop_requested && self.pending_op.is_none()
    }

    pub(crate) fn can_schedule_at(&self, now: Instant) -> bool {
        self.can_schedule()
            && !matches!(
                (self.status, self.pause_reason),
                (TwapStatus::Paused, Some(TwapPauseReason::StaleMarketData))
            )
            && self.status_check_cloid.is_none()
            && self.next_slice_due <= now
            && self.paused_until.is_none_or(|until| until <= now)
    }

    pub(crate) fn needs_timer_tick(&self) -> bool {
        !self.status.is_terminal()
            && ((!self.stop_requested && self.pending_op.is_none())
                || self.reconciliation_deadline.is_some())
    }

    /// Pure predicate so the timeout policy can be unit-tested against
    /// arbitrary deadlines without constructing a full TwapOrder.
    pub(crate) fn reconciliation_timed_out(deadline: Option<Instant>, now: Instant) -> bool {
        deadline.is_some_and(|d| now >= d)
    }

    pub(crate) fn retry_delay(retry_count: u32) -> Duration {
        let exponent = retry_count.saturating_sub(1).min(8);
        let multiplier = 1_u32.checked_shl(exponent).unwrap_or(u32::MAX);
        TWAP_RETRY_BASE_DELAY
            .saturating_mul(multiplier)
            .min(TWAP_RETRY_MAX_DELAY)
    }

    pub(crate) fn next_slice_size(&mut self) -> Option<f64> {
        let remaining_size = positive_finite_value(self.remaining_size)?;
        let remaining_slots = self.slice_count.saturating_sub(self.slices_attempted);
        if remaining_slots == 0 {
            return None;
        }
        if remaining_slots == 1 {
            return Some(remaining_size);
        }

        let base = remaining_size / f64::from(remaining_slots);
        let size = if self.randomize {
            let unit = next_random_unit(&mut self.random_seed);
            let factor = 1.0 - TWAP_RANDOM_JITTER + unit * TWAP_RANDOM_JITTER * 2.0;
            base * factor
        } else {
            base
        };
        let size = size.clamp(f64::MIN_POSITIVE, remaining_size);
        finite_value(size)
    }

    pub(crate) fn schedule_after_attempt(&mut self, now: Instant) {
        if self.remaining_size <= 0.0 {
            self.clear_pause();
            self.status = TwapStatus::Completed;
            self.push_event(
                TwapEventKind::Completed,
                "TWAP completed".to_string(),
                false,
            );
            return;
        }
        let remaining_slots = self.slice_count.saturating_sub(self.slices_attempted);
        if remaining_slots == 0 {
            self.clear_pause();
            self.status = if self.filled_size > 0.0 {
                TwapStatus::CompletedPartial
            } else {
                TwapStatus::Stopped
            };
            let message = if self.filled_size > 0.0 {
                "TWAP ended with unfilled remainder".to_string()
            } else {
                "TWAP ended without fills".to_string()
            };
            self.push_event(TwapEventKind::Completed, message, false);
            return;
        }

        let remaining_time = self.ends_at.saturating_duration_since(now);
        if remaining_time.is_zero() {
            self.next_slice_due = now;
            self.status = TwapStatus::WaitingForMarket;
            return;
        }

        let nominal_delay = remaining_time / remaining_slots;
        let delay = if self.randomize {
            let unit = next_random_unit(&mut self.random_seed);
            let factor = 1.0 - TWAP_RANDOM_JITTER + unit * TWAP_RANDOM_JITTER * 2.0;
            scaled_duration(nominal_delay, factor)
        } else {
            nominal_delay
        };
        let future_min = TWAP_MIN_INTERVAL.saturating_mul(remaining_slots.saturating_sub(1));
        let max_delay = remaining_time.saturating_sub(future_min);
        let delay = clamp_duration(delay, TWAP_MIN_INTERVAL.min(remaining_time), max_delay);
        self.next_slice_due = now + delay;
        self.clear_pause();
        self.status = TwapStatus::WaitingForMarket;
    }
}

pub(super) fn twap_seed(id: u64, now: Instant) -> u64 {
    let nanos = now.elapsed().as_nanos() as u64;
    id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(nanos)
        .max(1)
}

fn next_random_unit(seed: &mut u64) -> f64 {
    let mut value = (*seed).max(1);
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *seed = value.max(1);
    (value as f64 / u64::MAX as f64).clamp(0.0, 1.0)
}

fn scaled_duration(duration: Duration, factor: f64) -> Duration {
    let Some(factor) = positive_finite_value(factor) else {
        return duration;
    };
    Duration::from_secs_f64(duration.as_secs_f64() * factor)
}

fn clamp_duration(value: Duration, min: Duration, max: Duration) -> Duration {
    if max < min {
        return max;
    }
    value.max(min).min(max)
}
