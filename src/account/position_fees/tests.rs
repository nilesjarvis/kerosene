use super::*;
use crate::account::PositionLeverage;

fn position(coin: &str, size: &str) -> Position {
    Position {
        coin: coin.to_string(),
        szi: size.to_string(),
        entry_px: "100".to_string(),
        position_value: "100".to_string(),
        unrealized_pnl: "0".to_string(),
        liquidation_px: None,
        leverage: PositionLeverage {
            leverage_type: "cross".to_string(),
            value: 10,
        },
        margin_used: "10".to_string(),
        cum_funding: None,
    }
}

fn fill(time: u64, start: &str, side: &str, size: &str, fee: &str) -> UserFill {
    UserFill {
        coin: "BTC".to_string(),
        px: "100".to_string(),
        sz: size.to_string(),
        side: side.to_string(),
        start_position: Some(start.to_string()),
        time,
        hash: None,
        tid: Some(time),
        oid: Some(time),
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: fee.to_string(),
        fee_token: Some("USDC".to_string()),
    }
}

#[test]
fn totals_opening_increases_partial_closes_and_rebates() {
    let fills = vec![
        fill(1, "0", "B", "2", "1.5"),
        fill(2, "2", "B", "1", "-0.25"),
        fill(3, "3", "A", "1", "0.5"),
    ];
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "2"), &fills, None),
        Some(1.75)
    );
}

#[test]
fn excludes_previous_closed_positions_and_other_markets() {
    let mut fills = vec![
        fill(1, "0", "B", "2", "10"),
        fill(2, "2", "A", "2", "10"),
        fill(3, "0", "A", "1", "0.5"),
    ];
    let mut other_market = fill(4, "0", "A", "1", "20");
    other_market.coin = "xyz:BTC".to_string();
    fills.push(other_market);
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "-1"), &fills, None),
        Some(0.5)
    );
    assert_eq!(
        derive_position_spent_fees(&position("xyz:BTC", "-1"), &fills, None),
        Some(20.0)
    );
}

#[test]
fn reversals_attribute_only_the_opening_fraction_in_either_direction() {
    for (start, side, live) in [("2", "A", "-1"), ("-2", "B", "1")] {
        // The old position's opening history is unnecessary after a flip.
        let fills = vec![fill(3, start, side, "3", "1.5")];
        assert_eq!(
            derive_position_spent_fees(&position("BTC", live), &fills, None),
            Some(0.5)
        );
    }
}

#[test]
fn missing_opening_or_middle_fills_never_report_a_partial_total() {
    let opening = fill(1, "0", "B", "1", "0.5");
    let latest = fill(3, "2", "B", "1", "0.5");
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "3"), std::slice::from_ref(&latest), None),
        None
    );
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "3"), &[opening, latest], None),
        None
    );
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "3"), &[], None),
        None
    );
}

#[test]
fn fills_ahead_of_or_behind_live_position_are_unavailable() {
    let fills = vec![fill(1, "0", "B", "2", "1")];
    for live in ["1", "3", "-2", "0", "NaN"] {
        assert_eq!(
            derive_position_spent_fees(&position("BTC", live), &fills, None),
            None
        );
    }
}

#[test]
fn zero_fees_and_net_rebates_are_valid_totals() {
    for (fee, expected) in [("0", 0.0), ("-0.25", -0.25)] {
        assert_eq!(
            derive_position_spent_fees(&position("BTC", "1"), &[fill(1, "0", "B", "1", fee)], None),
            Some(expected)
        );
    }
}

#[test]
fn duplicate_fills_are_counted_once_and_input_order_is_irrelevant() {
    let first = fill(1, "0", "B", "1", "0.5");
    let last = fill(2, "1", "B", "1", "0.5");
    let fills = vec![last.clone(), first.clone(), last, first];
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "2"), &fills, None),
        Some(1.0)
    );
}

#[test]
fn same_timestamp_fills_follow_position_chain_not_trade_ids() {
    let opening = fill(1, "0", "B", "1", "0.5");
    let mut increase = fill(2, "1", "B", "1", "0.75");
    increase.time = 1;
    let mut reduce = fill(3, "2", "A", "0.5", "0.25");
    reduce.time = 1;
    let fills = vec![reduce, opening, increase];
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "1.5"), &fills, None),
        Some(1.5)
    );
}

#[test]
fn ambiguous_same_timestamp_chain_is_unavailable() {
    let opening = fill(1, "0", "B", "1", "0.5");
    let mut reduce = fill(2, "2", "A", "1", "0.25");
    reduce.time = 1;
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "1"), &[opening, reduce], None),
        None
    );
}

#[test]
fn invalid_current_fill_values_or_missing_start_position_are_unavailable() {
    let valid = fill(1, "0", "B", "1", "0.5");
    for bad in ["NaN", "inf", "invalid"] {
        let mut invalid = valid.clone();
        invalid.fee = bad.to_string();
        assert_eq!(
            derive_position_spent_fees(&position("BTC", "1"), &[invalid], None),
            None
        );
        let mut invalid = valid.clone();
        invalid.start_position = Some(bad.to_string());
        assert_eq!(
            derive_position_spent_fees(&position("BTC", "1"), &[invalid], None),
            None
        );
    }
    let mut missing = valid.clone();
    missing.start_position = None;
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "1"), &[missing], None),
        None
    );
    let mut invalid = valid;
    invalid.side = "unknown".to_string();
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "1"), &[invalid], None),
        None
    );
}

#[test]
fn malformed_old_closed_history_does_not_invalidate_current_position() {
    let fills = vec![
        fill(1, "bad", "B", "bad", "bad"),
        fill(2, "0", "B", "1", "0.5"),
    ];
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "1"), &fills, None),
        Some(0.5)
    );
}

#[test]
fn tiny_positions_are_not_treated_as_flat_dust() {
    let fills = vec![
        fill(1, "0", "B", "0.00000001", "0.001"),
        fill(2, "0.00000001", "B", "0.00000001", "0.001"),
    ];
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "0.00000002"), &fills, None),
        Some(0.002)
    );
    assert_eq!(
        derive_position_spent_fees(&position("BTC", "0.00000002"), &fills[1..], None),
        None
    );
}

#[test]
fn spot_fees_convert_at_fill_price_and_reconcile_net_inventory() {
    let mut buy = fill(1, "0", "B", "2", "0.01");
    buy.coin = "@107".to_string();
    buy.fee_token = Some(" HYPE ".to_string());
    buy.px = "50".to_string();
    let mut sell = fill(2, "1.99", "A", "0.99", "0.25");
    sell.coin = "@107".to_string();
    assert_eq!(
        derive_position_spent_fees(&position("@107", "1"), &[buy, sell], Some("HYPE")),
        Some(0.75)
    );
}

#[test]
fn transferred_spot_balances_and_unknown_fee_tokens_are_unavailable() {
    let mut buy = fill(1, "0", "B", "2", "0.01");
    buy.coin = "@107".to_string();
    buy.fee_token = Some("HYPE".to_string());
    assert_eq!(
        derive_position_spent_fees(&position("@107", "3"), &[buy.clone()], Some("HYPE")),
        None
    );
    buy.fee_token = Some("UNKNOWN".to_string());
    assert_eq!(
        derive_position_spent_fees(&position("@107", "2"), &[buy], Some("HYPE")),
        None
    );
}

#[test]
fn outcome_fees_and_trimmed_dollar_tokens_are_supported() {
    let mut buy = fill(1, "0", "B", "5", "0.02");
    buy.coin = "#950".to_string();
    buy.fee_token = Some(" USDH ".to_string());
    assert_eq!(
        derive_position_spent_fees(&position("#950", "5"), &[buy], Some("#950")),
        Some(0.02)
    );
}

#[test]
fn settlement_fees_do_not_change_position_size() {
    let mut settlement = fill(2, "1", "B", "0", "0.25");
    settlement.dir = "Settlement".to_string();
    assert_eq!(
        derive_position_spent_fees(
            &position("BTC", "1"),
            &[fill(1, "0", "B", "1", "0.5"), settlement],
            None
        ),
        Some(0.75)
    );
}
