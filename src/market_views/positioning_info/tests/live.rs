use super::*;

#[test]
fn positioning_live_mark_prefers_fresh_mark_context() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.asset_ctx = Some(asset_ctx(Some("31"), Some("30.5")));
    instance.asset_ctx_updated_at_ms = Some(1_000);

    assert_eq!(positioning_live_mark(&instance, 2_000), Some(31.0));
}

#[test]
fn positioning_live_mark_rejects_stale_or_invalid_context() {
    let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
    instance.asset_ctx = Some(asset_ctx(Some("bad"), Some("30.5")));
    instance.asset_ctx_updated_at_ms = Some(1_000);

    assert_eq!(positioning_live_mark(&instance, 2_000), Some(30.5));
    instance.asset_ctx = Some(asset_ctx(Some("0"), Some("-1")));
    assert_eq!(positioning_live_mark(&instance, 2_000), None);
    assert_eq!(
        positioning_live_mark(&instance, 1_000 + POSITIONING_LIVE_MARK_MAX_AGE_MS + 1),
        None
    );
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
