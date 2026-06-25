use super::super::AssetContext;

fn asset_ctx(impact_pxs: Option<Vec<&str>>) -> AssetContext {
    AssetContext {
        funding: None,
        open_interest: None,
        oracle_px: None,
        mark_px: None,
        mid_px: None,
        prev_day_px: None,
        day_ntl_vlm: None,
        day_base_vlm: None,
        impact_pxs: impact_pxs.map(|values| values.into_iter().map(str::to_string).collect()),
    }
}

#[test]
fn impact_spread_uses_ask_minus_bid() {
    let ctx = asset_ctx(Some(vec!["100.25", "100.75"]));

    assert_eq!(ctx.impact_spread(), Some(0.5));
}

#[test]
fn impact_spread_ignores_missing_or_invalid_prices() {
    assert_eq!(asset_ctx(None).impact_spread(), None);
    assert_eq!(asset_ctx(Some(vec!["100.25"])).impact_spread(), None);
    assert_eq!(asset_ctx(Some(vec!["bad", "100.75"])).impact_spread(), None);
    assert_eq!(asset_ctx(Some(vec!["100.25", "NaN"])).impact_spread(), None);
    assert_eq!(
        asset_ctx(Some(vec!["100.75", "100.25"])).impact_spread(),
        None
    );
}

#[test]
fn asset_context_debug_redacts_market_payload() {
    let ctx = AssetContext {
        funding: Some("funding-secret".to_string()),
        open_interest: Some("open-interest-secret".to_string()),
        oracle_px: Some("oracle-secret".to_string()),
        mark_px: Some("mark-secret".to_string()),
        mid_px: Some("mid-secret".to_string()),
        prev_day_px: Some("prev-day-secret".to_string()),
        day_ntl_vlm: Some("notional-volume-secret".to_string()),
        day_base_vlm: Some("base-volume-secret".to_string()),
        impact_pxs: Some(vec![
            "impact-bid-secret".to_string(),
            "impact-ask-secret".to_string(),
        ]),
    };

    let rendered = format!("{ctx:?}");

    assert!(rendered.contains("AssetContext"));
    assert!(rendered.contains("has_funding: true"));
    assert!(rendered.contains("impact_pxs_count: Some(2)"));
    for secret in [
        "funding-secret",
        "open-interest-secret",
        "oracle-secret",
        "mark-secret",
        "mid-secret",
        "prev-day-secret",
        "notional-volume-secret",
        "base-volume-secret",
        "impact-bid-secret",
        "impact-ask-secret",
    ] {
        assert!(
            !rendered.contains(secret),
            "asset context Debug leaked {secret}"
        );
    }
}
