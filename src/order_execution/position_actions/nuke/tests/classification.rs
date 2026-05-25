use super::{
    DEFAULT_MARKET_SLIPPAGE, NukePositionClassification, NukeSkipReason, NukeSymbolInfo,
    classify_nuke_position, perp_sym,
};
use crate::api::MarketType;

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
    // Zero size passes the parser but `build_nuke_position_order` rejects.
    let result =
        classify_nuke_position(0.0, Some(perp_sym()), Some(100.0), DEFAULT_MARKET_SLIPPAGE);
    assert_eq!(
        result,
        NukePositionClassification::Skip(NukeSkipReason::OrderBuildFailed)
    );

    // Negative slippage is also rejected at build time.
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
    // OrderBuildFailed; that order matches what the user can act on.
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
