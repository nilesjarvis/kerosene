use super::*;
use std::time::Duration;

#[test]
fn hype_unstaking_status_uses_supplied_tick_clock_for_age() {
    let now = Instant::now();
    let state = HypeUnstakingQueueState {
        last_fetch: Some(now - Duration::from_secs(30)),
        ..Default::default()
    };
    assert_eq!(hype_unstaking_status_text(&state, now), "Updated just now");

    let state = HypeUnstakingQueueState {
        last_fetch: Some(now - Duration::from_secs(125)),
        ..Default::default()
    };
    assert_eq!(hype_unstaking_status_text(&state, now), "Updated 2m ago");
}

#[test]
fn hype_unstaking_status_handles_loading_and_error_states() {
    let now = Instant::now();

    assert_eq!(
        hype_unstaking_status_text(
            &HypeUnstakingQueueState {
                loading: true,
                ..Default::default()
            },
            now,
        ),
        "Loading queue..."
    );
    assert_eq!(
        hype_unstaking_status_text(
            &HypeUnstakingQueueState {
                loading: true,
                data: Some(Default::default()),
                ..Default::default()
            },
            now,
        ),
        "Refreshing..."
    );
    assert_eq!(
        hype_unstaking_status_text(
            &HypeUnstakingQueueState {
                error: Some("network".to_string()),
                ..Default::default()
            },
            now,
        ),
        "Load failed: network"
    );
    assert_eq!(
        hype_unstaking_status_text(
            &HypeUnstakingQueueState {
                error: Some("network".to_string()),
                data: Some(Default::default()),
                ..Default::default()
            },
            now,
        ),
        "Showing last good data; refresh failed: network"
    );
}
