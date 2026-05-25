use super::{
    ChaseLifecycle, ChaseVerificationReason, apply_open_order_to_chase, chase_order, open_order,
};

#[test]
fn open_order_sync_updates_chase_size_price_and_confirmation() {
    let mut chase = chase_order();
    let mut order = open_order(42, Some(false));
    order.sz = "0.25".to_string();
    order.limit_px = "101.5".to_string();

    assert_eq!(apply_open_order_to_chase(&mut chase, &order), Ok(false));

    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 101.5);
    assert_eq!(chase.current_price_wire, "101.5");
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
}

#[test]
fn open_order_sync_clamps_chase_size_to_unfilled_target() {
    let mut chase = chase_order();
    chase.filled_size = 0.9;
    let mut order = open_order(42, Some(false));
    order.sz = "0.2".to_string();

    assert_eq!(apply_open_order_to_chase(&mut chase, &order), Ok(true));

    assert!((chase.remaining_size - 0.1).abs() < 1e-12);
}

#[test]
fn open_order_sync_rejects_invalid_remaining_size() {
    let mut chase = chase_order();
    let mut order = open_order(42, Some(false));
    order.sz = "0".to_string();

    assert_eq!(apply_open_order_to_chase(&mut chase, &order), Err(()));
    assert_eq!(chase.remaining_size, 1.0);
}

#[test]
fn open_order_sync_keeps_desired_price_until_exchange_price_matches() {
    let mut chase = chase_order();
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase.desired_price = Some(101.0);

    let mut stale_order = open_order(42, Some(false));
    stale_order.sz = "0.25".to_string();
    stale_order.limit_px = "100".to_string();

    assert_eq!(
        apply_open_order_to_chase(&mut chase, &stale_order),
        Ok(false)
    );

    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 100.0);
    assert_eq!(chase.current_price_wire, "100");
    assert_eq!(chase.desired_price, Some(101.0));

    let mut confirmed_order = open_order(42, Some(false));
    confirmed_order.sz = "0.25".to_string();
    confirmed_order.limit_px = "101".to_string();

    assert_eq!(
        apply_open_order_to_chase(&mut chase, &confirmed_order),
        Ok(false)
    );

    assert_eq!(chase.current_price, 101.0);
    assert_eq!(chase.current_price_wire, "101");
    assert_eq!(chase.desired_price, None);
}
