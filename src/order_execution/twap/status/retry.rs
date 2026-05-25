use crate::twap_state::{TWAP_MAX_RETRY_ATTEMPTS, TwapOrder};

use std::time::Duration;

// ---------------------------------------------------------------------------
// TWAP Status Retry Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TwapStatusRetryDecision {
    Retry { attempt: u32, delay: Duration },
    Exhausted { attempt: u32 },
}

impl TwapStatusRetryDecision {
    pub(super) fn attempt(self) -> u32 {
        match self {
            Self::Retry { attempt, .. } | Self::Exhausted { attempt } => attempt,
        }
    }
}

pub(super) fn next_twap_status_retry(current_attempts: u32) -> TwapStatusRetryDecision {
    let attempt = current_attempts.saturating_add(1);
    if attempt >= TWAP_MAX_RETRY_ATTEMPTS {
        TwapStatusRetryDecision::Exhausted { attempt }
    } else {
        TwapStatusRetryDecision::Retry {
            attempt,
            delay: TwapOrder::retry_delay(attempt),
        }
    }
}

#[cfg(test)]
mod tests;
