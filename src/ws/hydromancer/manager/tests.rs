use super::*;
use std::time::Duration;

#[test]
fn hydromancer_read_remaining_decreases_then_saturates_at_zero() {
    let window = Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS);
    assert_eq!(hydromancer_read_remaining(Duration::ZERO), window);
    assert_eq!(
        hydromancer_read_remaining(Duration::from_secs(10)),
        window - Duration::from_secs(10)
    );
    assert_eq!(
        hydromancer_read_remaining(Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS)),
        Duration::ZERO
    );
    assert_eq!(
        hydromancer_read_remaining(Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS * 4)),
        Duration::ZERO
    );
}

#[test]
fn hydromancer_window_is_larger_than_the_app_level_stale_label() {
    // The status pane flags the feed as "Stale" after 75s; the reconnect
    // watchdog needs to fire later than that so the user sees the warning
    // before the connection is torn down.
    const APP_STALE_LABEL_SECS: u64 = 75;
    const _: () = assert!(HYDROMANCER_READ_TIMEOUT_SECS > APP_STALE_LABEL_SECS);
}
