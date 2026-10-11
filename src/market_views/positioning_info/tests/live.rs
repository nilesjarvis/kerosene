use super::*;

#[test]
fn positioning_live_mark_prefers_fresh_mark_context() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.asset_ctx = Some(asset_ctx(Some("31"), Some("30.5")));
    instance.asset_ctx_updated_at_ms = Some(1_000);

    assert_eq!(positioning_live_mark(&instance, 2_000), Some(31.0));
}

#[test]
fn positioning_live_mark_stays_available_between_clock_ticks() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.asset_ctx = Some(asset_ctx(Some("31"), Some("30.5")));
    let deltas = [delta("synthetic-trader", 10.0, 2.0)];

    for tick_ms in [1_000, 2_000, 3_000] {
        // A websocket receipt arrives between cached one-second view ticks.
        instance.asset_ctx_updated_at_ms = Some(tick_ms + 500);
        for view_ms in [tick_ms, tick_ms + 1_000] {
            let mark = positioning_live_mark(&instance, view_ms);
            assert_eq!(mark, Some(31.0));
            let flow = positioning_flow_data(&deltas, mark, 10);
            assert!(flow.usd_available);
            assert_eq!(flow.rows[0].delta_usd, 62.0);
        }
    }
}

#[test]
fn positioning_live_mark_rejects_stale_or_invalid_context() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.asset_ctx = Some(asset_ctx(Some("bad"), Some("30.5")));
    instance.asset_ctx_updated_at_ms = Some(1_000);

    assert_eq!(positioning_live_mark(&instance, 2_000), Some(30.5));
    assert_eq!(
        positioning_live_mark(&instance, 1_000 + POSITIONING_LIVE_MARK_MAX_AGE_MS),
        Some(30.5)
    );
    assert_eq!(
        positioning_live_mark(&instance, 1_000 + POSITIONING_LIVE_MARK_MAX_AGE_MS + 1),
        None
    );
    instance.asset_ctx = Some(asset_ctx(Some("0"), Some("-1")));
    assert_eq!(positioning_live_mark(&instance, 2_000), None);
    instance.asset_ctx_updated_at_ms = None;
    assert_eq!(positioning_live_mark(&instance, 2_000), None);
}

#[test]
fn positioning_upnl_keeps_the_mark_through_partial_updates_until_it_expires() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    let position = sample_position();
    instance.update_asset_context(asset_ctx(Some("31"), Some("31.5")), 1_500);

    for (tick_ms, partial) in [
        (2_000, asset_ctx(None, Some("31.5"))),
        (3_000, asset_ctx(Some("bad"), Some("31.5"))),
        (4_000, asset_ctx(Some("NaN"), None)),
        (5_000, asset_ctx(None, None)),
    ] {
        instance.update_asset_context(partial, tick_ms + 500);
        assert_eq!(instance.asset_ctx_updated_at_ms, Some(1_500));
        for clock_ms in [tick_ms, tick_ms + 1_000] {
            let mark = positioning_live_mark(&instance, clock_ms);
            assert_eq!(mark, Some(31.0));
            assert_eq!(positioning_live_unrealized_pnl(&position, mark), Some(60.0));
        }
    }

    let expired_at = 1_500 + POSITIONING_LIVE_MARK_MAX_AGE_MS + 1;
    let mark = positioning_live_mark(&instance, expired_at);
    assert_eq!(mark, None);
    assert_eq!(positioning_live_unrealized_pnl(&position, mark), None);

    instance.update_asset_context(asset_ctx(Some("32"), Some("32.5")), expired_at + 500);
    let mark = positioning_live_mark(&instance, expired_at);
    assert_eq!(positioning_live_unrealized_pnl(&position, mark), Some(70.0));
}

#[test]
fn positioning_can_start_with_mid_then_upgrade_to_mark() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.update_asset_context(asset_ctx(None, Some("31.5")), 1_000);
    assert_eq!(positioning_live_mark(&instance, 1_000), Some(31.5));
    instance.update_asset_context(asset_ctx(Some("31"), Some("31.5")), 2_000);
    assert_eq!(positioning_live_mark(&instance, 2_000), Some(31.0));
}

#[test]
fn positioning_upnl_rejects_nonfinite_calculations() {
    let mut position = sample_position();
    position.size = f64::MAX;
    assert_eq!(positioning_live_unrealized_pnl(&position, Some(31.0)), None);
    position.size = 10.0;
    position.entry_price = f64::NAN;
    assert_eq!(positioning_live_unrealized_pnl(&position, Some(31.0)), None);
}

#[test]
fn positioning_live_row_values_use_mark_without_mutating_size() {
    let position = sample_position();

    assert_eq!(
        positioning_live_notional(&position, Some(31.0)),
        Some(310.0)
    );
    assert_eq!(
        positioning_live_unrealized_pnl(&position, Some(31.0)),
        Some(60.0)
    );
}
