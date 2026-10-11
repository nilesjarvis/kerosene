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
