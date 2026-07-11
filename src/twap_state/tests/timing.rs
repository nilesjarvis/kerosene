use super::{
    TWAP_RECONCILIATION_TIMEOUT, TwapOrder, TwapPauseReason, TwapStatus, test_twap_order,
    twap_child_cloid,
};

use std::time::{Duration, Instant};

#[test]
fn twap_child_cloid_is_stable_128_bit_hex() {
    let first = twap_child_cloid("0xabc", 7, 1_000, 3);
    let second = twap_child_cloid("0xabc", 7, 1_000, 3);
    let different = twap_child_cloid("0xabc", 7, 1_000, 4);

    assert_eq!(first, second);
    assert_ne!(first, different);
    assert_eq!(first.len(), 34);
    assert!(first.starts_with("0x"));
    assert!(first[2..].chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn paused_status_check_blocks_scheduling_until_reconciled() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 1.0, false, 2);
    twap.pause(
        TwapPauseReason::StatusUnknown,
        Some(now),
        "checking".to_string(),
        true,
    );
    twap.status_check_cloid = Some(twap_child_cloid("0xabc", 1, 1_000, 1));

    assert!(!twap.can_schedule_at(now));

    twap.status_check_cloid = None;
    assert!(twap.can_schedule_at(now));
}

#[test]
fn stale_market_data_pause_blocks_scheduling_until_cleared() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 1.0, false, 2);
    twap.pause(
        TwapPauseReason::StaleMarketData,
        None,
        "stale market data".to_string(),
        true,
    );

    assert!(!twap.can_schedule_at(now));

    twap.clear_pause();
    assert!(twap.can_schedule_at(now));
}

#[test]
fn stopped_twap_with_reconciliation_deadline_still_needs_timer_tick() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 1.0, false, 2);

    assert!(twap.needs_timer_tick());

    twap.stop_requested = true;
    assert!(!twap.needs_timer_tick());

    twap.reconciliation_deadline = Some(now);
    assert!(twap.needs_timer_tick());

    twap.status = TwapStatus::Stopped;
    assert!(!twap.needs_timer_tick());
}

#[test]
fn terminal_twaps_cannot_schedule_or_request_timer_ticks() {
    let now = Instant::now();

    for status in [
        TwapStatus::Stopped,
        TwapStatus::Completed,
        TwapStatus::CompletedPartial,
        TwapStatus::Error,
    ] {
        let mut twap = test_twap_order(now, 1.0, false, 2);
        twap.status = status;
        twap.next_slice_due = now;

        assert!(!twap.can_schedule());
        assert!(!twap.can_schedule_at(now));
        assert!(!twap.needs_timer_tick());
    }
}

#[test]
fn retry_delay_exponentially_backs_off_and_caps() {
    assert_eq!(TwapOrder::retry_delay(1), Duration::from_secs(2));
    assert_eq!(TwapOrder::retry_delay(2), Duration::from_secs(4));
    assert_eq!(TwapOrder::retry_delay(10), Duration::from_secs(60));
}

#[test]
fn reconciliation_timed_out_predicate_handles_none_and_boundary() {
    let now = Instant::now();

    // No deadline armed: never timed out.
    assert!(!TwapOrder::reconciliation_timed_out(None, now));

    // Deadline in the future: not yet timed out.
    assert!(!TwapOrder::reconciliation_timed_out(
        Some(now + Duration::from_millis(1)),
        now
    ));

    // Deadline exactly at `now`: counts as timed out so the watchdog
    // fires on the first reconcile after expiry rather than one tick later.
    assert!(TwapOrder::reconciliation_timed_out(Some(now), now));

    // Deadline in the past: timed out.
    assert!(TwapOrder::reconciliation_timed_out(
        Some(now - Duration::from_secs(1)),
        now
    ));
}

#[test]
fn reconciliation_timeout_is_long_enough_to_absorb_typical_indexer_lag() {
    // The exchange's `account.fills` endpoint has been observed to lag
    // a few seconds behind `orderStatus` under normal conditions. The
    // timeout must be loose enough that healthy operation doesn't
    // trip the watchdog. 60s is a generous floor; anything shorter
    // would frequently false-positive during minor indexer hiccups.
    const MIN_HEALTHY_TIMEOUT: Duration = Duration::from_secs(30);
    const _: () = {
        // Const-evaluated comparison so a future tightening of the
        // constant fails to compile rather than silently producing
        // flaky terminal errors in production.
        assert!(TWAP_RECONCILIATION_TIMEOUT.as_secs() >= MIN_HEALTHY_TIMEOUT.as_secs());
    };
}
