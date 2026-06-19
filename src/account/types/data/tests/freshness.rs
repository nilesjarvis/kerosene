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

#[test]
fn open_order_action_freshness_tracks_dex_lanes_independently() {
    let mut data = account_data_snapshot(1_000);
    let fresh_now = 1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS;
    let stale_now = fresh_now + 1;

    assert!(data.is_fresh_for_open_order_action_for_symbol("BTC", fresh_now));
    assert!(data.is_fresh_for_open_order_action_for_symbol("flx:BTC", fresh_now));

    data.mark_open_orders_fetched_at_for_dex(" FLX ", stale_now);

    assert!(data.is_fresh_for_open_order_action_for_symbol("flx:BTC", stale_now));
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("flx:BTC", stale_now),
        Some(0)
    );
    assert!(!data.is_fresh_for_open_order_action_for_symbol("BTC", stale_now));
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("BTC", stale_now),
        Some(AccountData::POSITION_ACTION_MAX_AGE_MS + 1)
    );

    data.mark_open_orders_fetched_at_for_dex("", stale_now);

    assert!(data.is_fresh_for_open_order_action_for_symbol("BTC", stale_now));

    let full_snapshot_now = stale_now + AccountData::POSITION_ACTION_MAX_AGE_MS + 1;
    data.mark_open_orders_fetched_at(full_snapshot_now);

    assert!(data.is_fresh_for_open_order_action_for_symbol("flx:BTC", full_snapshot_now));
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("flx:BTC", full_snapshot_now),
        Some(0)
    );
}

#[test]
fn scoped_hip3_open_order_freshness_does_not_cover_main_or_other_dexes() {
    let mut data = account_data_snapshot(1_000);
    data.fetch_scope = AccountDataFetchScope::hip3_dex("flx");

    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("BTC", 1_000),
        None
    );
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("xyz:BTC", 1_000),
        None
    );
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("flx:BTC", 1_000),
        Some(0)
    );
    assert!(!data.is_fresh_for_open_order_action_for_symbol("BTC", 1_000));
    assert!(!data.is_fresh_for_open_order_action_for_symbol("xyz:BTC", 1_000));
    assert!(data.is_fresh_for_open_order_action_for_symbol("flx:BTC", 1_000));
}

#[test]
fn scoped_hip3_positions_refresh_preserves_prior_open_order_lane_timestamp() {
    let mut data = account_data_snapshot(1_000);
    data.fetch_scope = AccountDataFetchScope::hip3_dex("flx");
    let stale_now = 1_000 + AccountData::POSITION_ACTION_MAX_AGE_MS + 1;

    data.mark_positions_fetched_at(stale_now);

    assert!(data.is_fresh_for_position_action(stale_now));
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("flx:BTC", stale_now),
        Some(AccountData::POSITION_ACTION_MAX_AGE_MS + 1)
    );
    assert!(!data.is_fresh_for_open_order_action_for_symbol("flx:BTC", stale_now));
    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("BTC", stale_now),
        None
    );
}

#[test]
fn incomplete_open_orders_do_not_fallback_to_account_fetch_timestamp() {
    let mut data = account_data_snapshot(1_000);
    data.fetch_scope = AccountDataFetchScope::hip3_dex("flx");
    data.completeness.open_orders_complete = false;

    assert_eq!(
        data.open_order_action_snapshot_age_ms_for_symbol("flx:BTC", 1_000),
        None
    );
    assert!(!data.is_fresh_for_open_order_action_for_symbol("flx:BTC", 1_000));
}
