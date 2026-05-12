use super::{
    NukePlan, NukePositionClassification, NukeSkipReason, NukeSymbolInfo,
    build_nuke_position_order, classify_nuke_position, parse_nuke_position_size,
};
use crate::api::MarketType;
use crate::order_execution::pricing::DEFAULT_MARKET_SLIPPAGE_PCT;

const DEFAULT_MARKET_SLIPPAGE: f64 = DEFAULT_MARKET_SLIPPAGE_PCT / 100.0;

fn perp_sym() -> NukeSymbolInfo {
    NukeSymbolInfo {
        asset_index: 7,
        sz_decimals: 4,
        market_type: MarketType::Perp,
    }
}

#[test]
fn nuke_order_closes_long_with_sell_market_price() {
    let order =
        build_nuke_position_order(7, 4, 100.0, 2.5, DEFAULT_MARKET_SLIPPAGE).expect("valid order");

    assert_eq!(order.asset, 7);
    assert!(!order.is_buy);
    assert_eq!(order.price, "99");
    assert_eq!(order.size, "2.5");
}

#[test]
fn nuke_order_closes_short_with_buy_market_price() {
    let order =
        build_nuke_position_order(8, 4, 100.0, -2.5, DEFAULT_MARKET_SLIPPAGE).expect("valid order");

    assert_eq!(order.asset, 8);
    assert!(order.is_buy);
    assert_eq!(order.price, "101");
    assert_eq!(order.size, "2.5");
}

#[test]
fn nuke_order_rejects_zero_or_nonfinite_inputs() {
    assert!(build_nuke_position_order(7, 4, 100.0, 0.0, DEFAULT_MARKET_SLIPPAGE).is_none());
    assert!(build_nuke_position_order(7, 4, 0.0, 2.5, DEFAULT_MARKET_SLIPPAGE).is_none());
    assert!(build_nuke_position_order(7, 4, f64::NAN, 2.5, DEFAULT_MARKET_SLIPPAGE).is_none());
    assert!(
        build_nuke_position_order(7, 4, 100.0, f64::INFINITY, DEFAULT_MARKET_SLIPPAGE).is_none()
    );
    assert!(build_nuke_position_order(7, 4, 100.0, 2.5, f64::NAN).is_none());
    assert!(build_nuke_position_order(7, 4, 100.0, 2.5, -0.01).is_none());
}

#[test]
fn nuke_position_size_parser_rejects_malformed_sizes_instead_of_zeroing_them() {
    assert_eq!(parse_nuke_position_size("BTC", "2.5"), Ok(Some(2.5)));
    assert_eq!(parse_nuke_position_size("BTC", "0"), Ok(None));

    assert!(parse_nuke_position_size("BTC", "not-a-number").is_err());
    assert!(parse_nuke_position_size("BTC", "NaN").is_err());
    assert!(parse_nuke_position_size("BTC", "inf").is_err());
}

#[test]
fn classifier_emits_order_for_priceable_perp_position() {
    let result =
        classify_nuke_position(2.5, Some(perp_sym()), Some(100.0), DEFAULT_MARKET_SLIPPAGE);
    let NukePositionClassification::Order(order) = result else {
        panic!("expected Order, got {result:?}");
    };
    assert_eq!(order.asset, 7);
    assert!(!order.is_buy);
}

#[test]
fn classifier_skips_unknown_asset_when_symbol_metadata_is_missing() {
    let result = classify_nuke_position(2.5, None, Some(100.0), DEFAULT_MARKET_SLIPPAGE);
    assert_eq!(
        result,
        NukePositionClassification::Skip(NukeSkipReason::UnknownAsset)
    );
}

#[test]
fn classifier_skips_non_perp_markets() {
    let spot = NukeSymbolInfo {
        market_type: MarketType::Spot,
        ..perp_sym()
    };
    let result = classify_nuke_position(2.5, Some(spot), Some(100.0), DEFAULT_MARKET_SLIPPAGE);
    assert_eq!(
        result,
        NukePositionClassification::Skip(NukeSkipReason::NonPerp)
    );
}

#[test]
fn classifier_skips_when_no_mid_price_is_resolvable() {
    let result = classify_nuke_position(2.5, Some(perp_sym()), None, DEFAULT_MARKET_SLIPPAGE);
    assert_eq!(
        result,
        NukePositionClassification::Skip(NukeSkipReason::NoMidPrice)
    );
}

#[test]
fn classifier_skips_when_order_construction_rejects_inputs() {
    // Zero size — passes the parser but `build_nuke_position_order` rejects.
    let result =
        classify_nuke_position(0.0, Some(perp_sym()), Some(100.0), DEFAULT_MARKET_SLIPPAGE);
    assert_eq!(
        result,
        NukePositionClassification::Skip(NukeSkipReason::OrderBuildFailed)
    );

    // Negative slippage — also rejected at build time.
    let bad_slippage = classify_nuke_position(2.5, Some(perp_sym()), Some(100.0), -0.01);
    assert_eq!(
        bad_slippage,
        NukePositionClassification::Skip(NukeSkipReason::OrderBuildFailed)
    );
}

#[test]
fn classifier_decision_order_matches_user_facing_priority() {
    // When multiple conditions fail simultaneously, the user sees the
    // earliest failure. UnknownAsset > NonPerp > NoMidPrice >
    // OrderBuildFailed — that order matches what the user can act on
    // (a missing symbol is more diagnostic than a missing mid).
    assert_eq!(
        classify_nuke_position(2.5, None, None, DEFAULT_MARKET_SLIPPAGE),
        NukePositionClassification::Skip(NukeSkipReason::UnknownAsset)
    );
    let spot = NukeSymbolInfo {
        market_type: MarketType::Spot,
        ..perp_sym()
    };
    assert_eq!(
        classify_nuke_position(2.5, Some(spot), None, DEFAULT_MARKET_SLIPPAGE),
        NukePositionClassification::Skip(NukeSkipReason::NonPerp)
    );
}

#[test]
fn plan_skip_list_renders_each_position_with_its_reason() {
    let plan = NukePlan {
        ready: vec![],
        skipped: vec![
            ("SHIB".to_string(), NukeSkipReason::NoMidPrice),
            ("BTC-SPOT".to_string(), NukeSkipReason::NonPerp),
        ],
    };
    assert_eq!(
        plan.format_skip_list(),
        "SHIB (no mid price), BTC-SPOT (not a perpetual market)"
    );
}

#[test]
fn plan_ready_list_renders_each_coin_in_order() {
    let order =
        build_nuke_position_order(1, 2, 100.0, 1.0, DEFAULT_MARKET_SLIPPAGE).expect("order");
    let plan = NukePlan {
        ready: vec![
            ("BTC".to_string(), order.clone()),
            ("ETH".to_string(), order.clone()),
            ("SOL".to_string(), order),
        ],
        skipped: vec![],
    };
    assert_eq!(plan.format_ready_list(), "BTC, ETH, SOL");
}

#[test]
fn plan_is_empty_iff_both_lists_are_empty() {
    assert!(NukePlan::default().is_empty());
    let with_skip = NukePlan {
        ready: vec![],
        skipped: vec![("X".to_string(), NukeSkipReason::UnknownAsset)],
    };
    assert!(!with_skip.is_empty());
}
