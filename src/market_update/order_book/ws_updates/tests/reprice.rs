use super::*;

#[test]
fn chase_reprices_only_when_symbol_active_ready_and_price_changed() {
    let chase = chase();

    let now = Instant::now();

    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(99.0), now));
    assert!(!chase_should_reprice(&chase, "ETH", "BTC", Some(99.0), now));
    assert!(!chase_should_reprice(&chase, "BTC", "ETH", Some(99.0), now));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", None, now));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(98.0), now));
    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(97.0), now));
}

#[test]
fn sell_chase_reprices_only_toward_lower_prices() {
    let mut chase = chase();
    chase.is_buy = false;
    chase.current_price = 172.0;
    chase.current_price_wire = "172".to_string();

    let now = Instant::now();

    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(171.8), now));
    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(172.2),
        now
    ));
}

#[test]
fn chase_reprice_waits_while_operation_is_in_flight() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Modifying { oid: 42 };

    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(99.0),
        Instant::now()
    ));
}

#[test]
fn chase_reprice_waits_after_stop_is_requested() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: crate::signing::ChaseStopPhase::Canceling { oid: 42 },
    };

    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(99.0),
        Instant::now()
    ));
}

#[test]
fn chase_reprice_compares_rounded_wire_price() {
    let mut chase = chase();
    chase.sz_decimals = 2;
    chase.current_price = 100.0;
    chase.current_price_wire = "100".to_string();

    assert!(!chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(100.001),
        Instant::now()
    ));
    assert!(chase_should_reprice(
        &chase,
        "BTC",
        "BTC",
        Some(101.0),
        Instant::now()
    ));
}

#[test]
fn chase_reprice_waits_for_minimum_interval() {
    let mut chase = chase();
    let now = Instant::now();
    let min_interval = Duration::from_secs(1);
    chase.last_reprice_at = Some(now - min_interval + Duration::from_millis(1));

    assert!(!chase_should_reprice(&chase, "BTC", "BTC", Some(99.0), now));

    chase.last_reprice_at = Some(now - min_interval);
    assert!(chase_should_reprice(&chase, "BTC", "BTC", Some(99.0), now));
}
