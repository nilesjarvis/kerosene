use super::*;

#[test]
fn position_action_snapshot_is_fresh_only_within_cutoff() {
    let data = account_data_snapshot(1_000);

    assert!(data.is_fresh_for_position_action(1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS));
    assert!(
        !data.is_fresh_for_position_action(1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS + 1)
    );
    assert!(!data.is_fresh_for_position_action(999));
}

#[test]
fn positions_refresh_does_not_refresh_open_order_action_snapshot() {
    let mut data = account_data_snapshot(1_000);
    let fresh_now = 1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS;
    let stale_now = fresh_now + 1;

    assert!(data.is_fresh_for_open_order_action(fresh_now));

    data.mark_positions_fetched_at(stale_now);

    assert!(data.is_fresh_for_position_action(stale_now));
    assert_eq!(
        data.open_order_action_snapshot_age_ms(stale_now),
        Some(AccountData::POSITION_ACTION_MAX_AGE_MS + 1)
    );
    assert!(!data.is_fresh_for_open_order_action(stale_now));

    data.mark_open_orders_fetched_at(stale_now);

    assert_eq!(data.open_order_action_snapshot_age_ms(stale_now), Some(0));
    assert!(data.is_fresh_for_open_order_action(stale_now));
}
