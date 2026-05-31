use crate::account::AssetContext;

use super::*;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

fn asset_ctx(impact_pxs: Option<Vec<&str>>) -> AssetContext {
    AssetContext {
        funding: None,
        open_interest: None,
        oracle_px: None,
        mark_px: None,
        mid_px: None,
        prev_day_px: None,
        day_ntl_vlm: None,
        impact_pxs: impact_pxs.map(|values| values.into_iter().map(str::to_string).collect()),
    }
}

#[test]
fn impact_spread_uses_ask_minus_bid() {
    let ctx = asset_ctx(Some(vec!["100.25", "100.75"]));

    assert_eq!(impact_spread(&ctx), Some(0.5));
}

#[test]
fn impact_spread_ignores_missing_or_invalid_prices() {
    assert_eq!(impact_spread(&asset_ctx(None)), None);
    assert_eq!(impact_spread(&asset_ctx(Some(vec!["100.25"]))), None);
    assert_eq!(impact_spread(&asset_ctx(Some(vec!["bad", "100.75"]))), None);
    assert_eq!(impact_spread(&asset_ctx(Some(vec!["100.25", "NaN"]))), None);
    assert_eq!(
        impact_spread(&asset_ctx(Some(vec!["100.75", "100.25"]))),
        None
    );
}

#[test]
fn record_spread_pushes_newest_point_and_trims_expired_tail() {
    let now = Instant::now();
    let mut history = VecDeque::from([
        (now - Duration::from_secs(120), 0.2),
        (now - Duration::from_secs(301), 0.3),
    ]);

    record_asset_context_spread(&mut history, &asset_ctx(Some(vec!["10", "10.5"])), now);

    assert_eq!(history.len(), 2);
    assert_eq!(history.front().map(|(_, spread)| *spread), Some(0.5));
    assert_eq!(history.back().map(|(_, spread)| *spread), Some(0.2));
}
