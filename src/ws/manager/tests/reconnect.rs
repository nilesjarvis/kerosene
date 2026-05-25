use super::*;

#[test]
fn default_policy_holds_known_exchange_constants() {
    assert_eq!(EXCHANGE_WS_RECONNECT_POLICY.base_delay_secs, 1);
    assert_eq!(EXCHANGE_WS_RECONNECT_POLICY.max_delay_secs, 60);
    assert_eq!(EXCHANGE_WS_RECONNECT_POLICY.reset_after_secs, 30);
}

#[test]
fn next_delay_doubles_until_capped() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    assert_eq!(policy.next_delay(0), policy.base_delay_secs);
    assert_eq!(policy.next_delay(1), 2);
    assert_eq!(policy.next_delay(32), policy.max_delay_secs);
    assert_eq!(
        policy.next_delay(policy.max_delay_secs),
        policy.max_delay_secs
    );
}

#[test]
fn after_disconnect_resets_when_connection_was_stable() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let stable_for = Duration::from_secs(policy.reset_after_secs);

    let (delay, next) = policy.after_disconnect(16, stable_for);

    assert_eq!(delay, policy.base_delay_secs);
    assert_eq!(next, policy.next_delay(policy.base_delay_secs));
}

#[test]
fn after_disconnect_keeps_backing_off_after_quick_failure() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let quick = Duration::from_secs(1);

    let (delay, next) = policy.after_disconnect(8, quick);

    assert_eq!(delay, 8);
    assert_eq!(next, 16);
}

#[test]
fn policy_math_works_with_arbitrary_values() {
    // Tight policy: 1..=10s window, resets if connection survived 5s.
    let tight = ReconnectPolicy {
        base_delay_secs: 1,
        max_delay_secs: 10,
        reset_after_secs: 5,
    };

    assert_eq!(tight.next_delay(0), 1);
    assert_eq!(tight.next_delay(4), 8);
    assert_eq!(tight.next_delay(8), 10);
    assert_eq!(tight.next_delay(50), 10);

    let (delay, _) = tight.after_disconnect(8, Duration::from_secs(10));
    assert_eq!(delay, 1, "stable connection should reset to base");

    let (delay, _) = tight.after_disconnect(8, Duration::from_secs(1));
    assert_eq!(delay, 8, "unstable connection should hold backoff");
}
