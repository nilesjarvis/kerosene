use super::*;
use crate::account::{Position, PositionLeverage};

fn asset_position(szi: &str, value: &str, upnl: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: "BTC".to_string(),
            szi: szi.to_string(),
            entry_px: "100".to_string(),
            position_value: value.to_string(),
            unrealized_pnl: upnl.to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "1".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

#[test]
fn tracker_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_tracker_number(" 1.25 "), Some(1.25));
    assert_eq!(parse_tracker_number("-2"), Some(-2.0));
    assert_eq!(parse_tracker_number("bad"), None);
    assert_eq!(parse_tracker_number("NaN"), None);
    assert_eq!(parse_tracker_number("inf"), None);
}

#[test]
fn wallet_tracker_snapshot_sums_valid_position_metrics() {
    let snapshot = build_wallet_tracker_snapshot(
        Some(100.0),
        Some(80.0),
        Some(25.0),
        vec![
            asset_position("2", "50", "4"),
            asset_position("-1", "-30", "-2"),
            asset_position("0", "100", "100"),
        ],
    );

    assert_eq!(snapshot.equity, Some(100.0));
    assert_eq!(snapshot.withdrawable, Some(80.0));
    assert_eq!(snapshot.margin_used_pct, Some(25.0));
    assert_eq!(snapshot.open_trade_count, Some(2));
    assert_eq!(snapshot.long_exposure, Some(50.0));
    assert_eq!(snapshot.short_exposure, Some(30.0));
    assert_eq!(snapshot.unrealized_pnl, Some(2.0));
}

#[test]
fn invalid_position_size_marks_tracker_aggregates_unknown() {
    let snapshot = build_wallet_tracker_snapshot(
        Some(100.0),
        Some(80.0),
        Some(25.0),
        vec![asset_position("bad", "50", "4")],
    );

    assert_eq!(snapshot.open_trade_count, None);
    assert_eq!(snapshot.long_exposure, None);
    assert_eq!(snapshot.short_exposure, None);
    assert_eq!(snapshot.unrealized_pnl, None);
}

#[test]
fn invalid_position_value_only_marks_exposure_unknown() {
    let snapshot = build_wallet_tracker_snapshot(
        Some(100.0),
        Some(80.0),
        Some(25.0),
        vec![asset_position("2", "bad", "4")],
    );

    assert_eq!(snapshot.open_trade_count, Some(1));
    assert_eq!(snapshot.long_exposure, None);
    assert_eq!(snapshot.short_exposure, Some(0.0));
    assert_eq!(snapshot.unrealized_pnl, Some(4.0));
}

#[test]
fn margin_used_pct_rejects_invalid_or_ambiguous_values() {
    assert_eq!(margin_used_pct(Some(100.0), Some(25.0)), Some(25.0));
    assert_eq!(margin_used_pct(Some(0.0), Some(0.0)), Some(0.0));
    assert_eq!(margin_used_pct(Some(0.0), Some(1.0)), None);
    assert_eq!(margin_used_pct(None, Some(1.0)), None);
    assert_eq!(margin_used_pct(Some(100.0), None), None);
}
