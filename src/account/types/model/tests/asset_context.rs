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
