use super::*;

#[test]
fn history_is_bounded_and_ordered_without_losing_rate_counts() {
    let mut log = ActivityLog::default();
    for _ in 0..HISTORY_LIMIT + 25 {
        log.record(
            ActivityEntry::new(Provider::Hyperliquid, ActivityKind::HttpSend, "allMids"),
            1,
            1000,
        );
    }
    let snapshot = log.snapshot(1);
    assert_eq!(snapshot.entries.len(), HISTORY_LIMIT);
    assert_eq!(
        snapshot.entries.front().expect("retained history").sequence,
        26
    );
    assert_eq!(
        snapshot.entries.back().expect("retained history").sequence,
        (HISTORY_LIMIT + 25) as u64
    );
    assert_eq!(
        snapshot.recent[Provider::All as usize].requests,
        (HISTORY_LIMIT + 25) as u64
    );
    assert_eq!(
        snapshot.recent[Provider::Hyperliquid as usize].requests,
        (HISTORY_LIMIT + 25) as u64
    );
    assert_eq!(snapshot.recent[Provider::Hydromancer as usize].requests, 0);
}

#[test]
fn rates_expire_while_idle_and_do_not_count_responses_as_requests() {
    let mut log = ActivityLog::default();
    log.record(
        ActivityEntry::new(Provider::Hyperliquid, ActivityKind::HttpSend, "allMids"),
        0,
        0,
    );
    log.record(
        ActivityEntry::new(
            Provider::Hyperliquid,
            ActivityKind::HttpResponse(429),
            "allMids",
        ),
        1,
        1000,
    );
    let mut frame = ActivityEntry::new(Provider::Hydromancer, ActivityKind::WsReceive, "l2Book");
    frame.bytes = Some(500);
    log.record(frame, 1, 1000);
    let snapshot = log.snapshot(59);
    let all = snapshot.recent[Provider::All as usize];
    assert_eq!(
        (all.requests, all.finished, all.errors, all.rate_limited),
        (1, 1, 1, 1)
    );
    assert_eq!((all.ws_received, all.ws_bytes), (1, 500));
    assert_eq!(log.snapshot(60).recent[Provider::All as usize].requests, 0);
    let idle = log.snapshot(61);
    assert_eq!(idle.recent[Provider::All as usize].ws_received, 0);
    assert_eq!(idle.recent[Provider::All as usize].errors, 0);
    assert_eq!(idle.totals[Provider::All as usize].requests, 1);
}

#[test]
fn unknown_operation_names_cannot_leak_arbitrary_values() {
    for value in [
        "0xprivate-wallet",
        "bearer-token",
        "https://private.example/?key=secret",
        "userFills\nsecret",
    ] {
        assert_eq!(safe_operation(value), "other");
    }
    assert_eq!(safe_operation("userFills"), "userFills");
}

#[test]
fn websocket_error_frames_count_as_receives_and_errors() {
    let mut log = ActivityLog::default();
    let mut frame =
        ActivityEntry::new(Provider::Hyperliquid, ActivityKind::WsReceiveError, "error");
    frame.bytes = Some(50);
    log.record(frame, 0, 0);
    let snapshot = log.snapshot(0);
    let counts = snapshot.recent[Provider::Hyperliquid as usize];
    assert_eq!(
        (
            counts.requests,
            counts.ws_received,
            counts.ws_bytes,
            counts.errors
        ),
        (0, 1, 50, 1)
    );
}
