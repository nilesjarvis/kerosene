use super::super::tests::{candle, details, live_trade, request, trade};
use super::*;

#[test]
fn overlapping_candles_include_candle_spanning_short_trade() {
    let metrics = journal_snapshot_metrics(
        &request(),
        &trade(true),
        Some(&details()),
        &[candle(0, 60_000, 95.0, 115.0, 110.0)],
    )
    .expect("metrics");

    assert_eq!(metrics.candle_count, 1);
    assert!((metrics.raw_asset_move - 0.10).abs() <= 1e-9);
}

#[test]
fn short_directional_metrics_are_inverted() {
    let metrics = journal_snapshot_metrics(
        &request(),
        &trade(false),
        Some(&details()),
        &[candle(0, 60_000, 95.0, 115.0, 110.0)],
    )
    .expect("metrics");

    assert!((metrics.directional_move + 0.10).abs() <= 1e-9);
    assert!((metrics.max_adverse_excursion + 0.15).abs() <= 1e-9);
    assert!((metrics.max_favorable_excursion - 0.05).abs() <= 1e-9);
}

#[test]
fn metrics_derive_from_entry_without_fills_for_live_position() {
    let metrics = journal_snapshot_metrics(
        &request(),
        &live_trade(),
        None,
        &[candle(0, 60_000, 95.0, 115.0, 110.0)],
    )
    .expect("metrics");

    assert!((metrics.entry_price - 100.0).abs() <= 1e-9);
    // Open position: the reference/exit price is the latest candle close.
    assert!((metrics.exit_price - 110.0).abs() <= 1e-9);
    assert!((metrics.raw_asset_move - 0.10).abs() <= 1e-9);
}
