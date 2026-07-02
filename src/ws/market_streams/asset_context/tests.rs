use super::*;

#[test]
fn active_asset_ctx_channel_accepts_perp_and_spot_variants() {
    assert!(is_active_asset_ctx_channel("activeAssetCtx"));
    assert!(is_active_asset_ctx_channel("activeSpotAssetCtx"));
    assert!(!is_active_asset_ctx_channel("l2Book"));
    assert!(!is_active_asset_ctx_channel("subscriptionResponse"));
}

#[test]
fn parse_active_asset_ctx_accepts_spot_pushes_on_the_spot_channel() {
    // Subscribing {"type":"activeAssetCtx","coin":"@107"} is acked, but the
    // server replies on channel "activeSpotAssetCtx" with a spot-shaped ctx
    // (no funding / openInterest / oraclePx).
    let msg = serde_json::json!({
        "channel": "activeSpotAssetCtx",
        "data": {
            "coin": "@107",
            "ctx": {
                "prevDayPx": "44.51",
                "dayNtlVlm": "72529712.1",
                "markPx": "44.85",
                "midPx": "44.855",
                "circulatingSupply": "270120439.63",
                "coin": "@107",
                "totalSupply": "999990491.63",
                "dayBaseVlm": "1624413.87"
            }
        }
    });
    let channel = msg["channel"].as_str().expect("channel");
    let ctx =
        parse_active_asset_ctx(channel, &msg["data"], "@107").expect("spot ctx must dispatch");
    assert_eq!(ctx.mid_px.as_deref(), Some("44.855"));
    assert_eq!(ctx.mark_px.as_deref(), Some("44.85"));
    assert_eq!(ctx.prev_day_px.as_deref(), Some("44.51"));
    assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("72529712.1"));
    assert_eq!(ctx.day_base_vlm.as_deref(), Some("1624413.87"));
    assert!(ctx.funding.is_none());
    assert!(ctx.open_interest.is_none());
    assert!(ctx.oracle_px.is_none());
}

#[test]
fn parse_active_asset_ctx_accepts_perp_pushes_on_the_perp_channel() {
    let msg = serde_json::json!({
        "channel": "activeAssetCtx",
        "data": {
            "coin": "BTC",
            "ctx": {
                "funding": "0.0000125",
                "openInterest": "688.11",
                "prevDayPx": "62912.0",
                "dayNtlVlm": "1471506129.2",
                "premium": "0.00031774",
                "oraclePx": "63251.0",
                "markPx": "63239.0",
                "midPx": "63239.5",
                "impactPxs": ["63239.0", "63240.0"],
                "dayBaseVlm": "23112.94"
            }
        }
    });
    let channel = msg["channel"].as_str().expect("channel");
    let ctx = parse_active_asset_ctx(channel, &msg["data"], "BTC").expect("perp ctx must dispatch");
    assert_eq!(ctx.funding.as_deref(), Some("0.0000125"));
    assert_eq!(ctx.mid_px.as_deref(), Some("63239.5"));
}

#[test]
fn parse_active_asset_ctx_rejects_other_coins_and_channels() {
    let data = serde_json::json!({
        "coin": "@107",
        "ctx": { "midPx": "44.855" }
    });
    assert!(parse_active_asset_ctx("activeSpotAssetCtx", &data, "@142").is_none());
    assert!(parse_active_asset_ctx("l2Book", &data, "@107").is_none());
    assert!(
        parse_active_asset_ctx(
            "activeSpotAssetCtx",
            &serde_json::json!({"coin": "@107"}),
            "@107"
        )
        .is_none()
    );
}
