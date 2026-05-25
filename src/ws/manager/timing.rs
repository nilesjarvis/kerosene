use std::time::Duration;

// ---------------------------------------------------------------------------
// Reconnect Timing
// ---------------------------------------------------------------------------

// Force a reconnect if no frame (data or pong) has arrived for this long.
// The manager pings every 30s, so 45s = "missed at least one heartbeat round-trip"
// and is the recovery path for half-open sockets left behind by VPN/NAT rebinds,
// where reads silently stall while writes still queue into the kernel buffer.
pub(super) const WS_READ_STALE_AFTER_SECS: u64 = 45;

pub(super) fn stale_read_remaining(last_rx_elapsed: Duration) -> Duration {
    Duration::from_secs(WS_READ_STALE_AFTER_SECS).saturating_sub(last_rx_elapsed)
}

pub(super) fn read_loop_timeout(stale_in: Duration, coalesce_due: Option<Duration>) -> Duration {
    coalesce_due.map_or(stale_in, |due| due.min(stale_in))
}

pub(super) const EXCHANGE_WS_RECONNECT_POLICY: ReconnectPolicy = ReconnectPolicy {
    base_delay_secs: 1,
    max_delay_secs: 60,
    reset_after_secs: 30,
};

/// Exponential-backoff parameters for a WebSocket reconnect loop. Lives as a
/// value so the policy math can be unit-tested with arbitrary values and so
/// per-feed policies don't have to drift apart as parallel module-level
/// constants.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReconnectPolicy {
    pub(crate) base_delay_secs: u64,
    pub(crate) max_delay_secs: u64,
    pub(crate) reset_after_secs: u64,
}

impl ReconnectPolicy {
    pub(crate) fn next_delay(&self, current_secs: u64) -> u64 {
        if current_secs < self.base_delay_secs {
            return self.base_delay_secs;
        }
        current_secs.saturating_mul(2).min(self.max_delay_secs)
    }

    /// Compute the next sleep, plus the delay to start from on the iteration
    /// after that. Resets to `base_delay_secs` when the just-closed connection
    /// stayed up long enough to count as healthy.
    pub(crate) fn after_disconnect(
        &self,
        current_secs: u64,
        connected_for: Duration,
    ) -> (u64, u64) {
        let delay_secs = if connected_for >= Duration::from_secs(self.reset_after_secs) {
            self.base_delay_secs
        } else {
            current_secs.clamp(self.base_delay_secs, self.max_delay_secs)
        };
        (delay_secs, self.next_delay(delay_secs))
    }
}
