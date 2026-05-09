use super::*;

#[test]
fn reconnect_delay_doubles_until_capped() {
    assert_eq!(next_reconnect_delay_secs(0), WS_RECONNECT_BASE_DELAY_SECS);
    assert_eq!(next_reconnect_delay_secs(1), 2);
    assert_eq!(next_reconnect_delay_secs(32), WS_RECONNECT_MAX_DELAY_SECS);
    assert_eq!(
        next_reconnect_delay_secs(WS_RECONNECT_MAX_DELAY_SECS),
        WS_RECONNECT_MAX_DELAY_SECS
    );
}

#[test]
fn reconnect_delay_resets_after_stable_connection() {
    let stable_for = std::time::Duration::from_secs(WS_RECONNECT_RESET_AFTER_SECS);
    let (delay, next) = reconnect_delay_after_disconnect(16, stable_for);

    assert_eq!(delay, WS_RECONNECT_BASE_DELAY_SECS);
    assert_eq!(
        next,
        next_reconnect_delay_secs(WS_RECONNECT_BASE_DELAY_SECS)
    );
}

#[test]
fn reconnect_delay_keeps_backing_off_after_quick_disconnect() {
    let quick_disconnect = std::time::Duration::from_secs(1);
    let (delay, next) = reconnect_delay_after_disconnect(8, quick_disconnect);

    assert_eq!(delay, 8);
    assert_eq!(next, 16);
}
