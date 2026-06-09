use super::*;
use crate::account::AssetContext;

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
fn asset_context_updates_current_chart_spread() {
    let mut instance = instance();

    instance.set_asset_context(Some(asset_ctx(Some(vec!["100.25", "100.75"]))));

    assert_eq!(instance.chart.current_spread, Some(0.5));
}

#[test]
fn asset_context_spread_history_keeps_last_ten_seconds() {
    let mut instance = instance();

    instance.set_asset_context_at(Some(asset_ctx(Some(vec!["100.25", "100.75"]))), 0);
    instance.set_asset_context_at(Some(asset_ctx(Some(vec!["100.25", "101.75"]))), 6_000);
    instance.set_asset_context_at(Some(asset_ctx(Some(vec!["100.25", "101.00"]))), 15_001);

    let bounds = instance
        .chart
        .spread_history_bounds()
        .expect("spread history bounds");
    crate::helpers::assert_close(bounds.0, 0.75);
    crate::helpers::assert_close(bounds.1, 1.5);
    assert_eq!(instance.chart.spread_history.len(), 2);
}

#[test]
fn asset_context_reset_clears_spread_and_history() {
    let mut instance = instance();
    instance.set_asset_context(Some(asset_ctx(Some(vec!["100.25", "100.75"]))));

    instance.set_asset_context(None);

    assert_eq!(instance.chart.current_spread, None);
    assert!(instance.chart.spread_history.is_empty());
}

#[test]
fn asset_context_without_impact_prices_keeps_spread_history() {
    let mut instance = instance();
    instance.set_asset_context_at(Some(asset_ctx(Some(vec!["100.25", "100.75"]))), 1_000);

    instance.set_asset_context_at(Some(asset_ctx(None)), 2_000);

    assert_eq!(instance.chart.current_spread, None);
    assert_eq!(instance.chart.spread_history.len(), 1);
    assert!(instance.chart.spread_history_bounds().is_some());
}

#[test]
fn asset_context_ignores_invalid_spreads() {
    let mut instance = instance();

    instance.set_asset_context(Some(asset_ctx(Some(vec!["100.75", "100.25"]))));

    assert_eq!(instance.chart.current_spread, None);
}
