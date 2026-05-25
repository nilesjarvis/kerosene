use super::*;

#[test]
fn stale_read_remaining_decreases_then_saturates_at_zero() {
    let window = Duration::from_secs(WS_READ_STALE_AFTER_SECS);
    assert_eq!(stale_read_remaining(Duration::from_secs(0)), window);
    assert_eq!(
        stale_read_remaining(Duration::from_secs(10)),
        window - Duration::from_secs(10)
    );
    assert_eq!(
        stale_read_remaining(Duration::from_secs(WS_READ_STALE_AFTER_SECS)),
        Duration::ZERO
    );
    assert_eq!(
        stale_read_remaining(Duration::from_secs(WS_READ_STALE_AFTER_SECS * 4)),
        Duration::ZERO
    );
}

#[test]
fn stale_window_exceeds_ping_interval() {
    // Pings fire every 30s; the watchdog must be loose enough to absorb a
    // single missed pong round-trip without triggering a reconnect.
    const _: () = assert!(WS_READ_STALE_AFTER_SECS > 30);
}
