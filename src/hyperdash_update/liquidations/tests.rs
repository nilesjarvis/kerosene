use super::planning::{
    LiquidationPlanContext, LiquidationRequestPlan, liquidation_mark_from_ctx,
    liquidation_request_coin, liquidation_request_key, liquidation_request_plan,
};

#[test]
fn liquidation_mark_parser_rejects_missing_nonpositive_or_nonfinite_values() {
    assert_eq!(liquidation_mark_from_ctx(Some("100.5"), None), Some(100.5));
    assert_eq!(liquidation_mark_from_ctx(None, None), None);
    assert_eq!(liquidation_mark_from_ctx(Some("0"), Some(90.0)), Some(90.0));
    assert_eq!(
        liquidation_mark_from_ctx(Some("-1"), Some(90.0)),
        Some(90.0)
    );
    assert_eq!(
        liquidation_mark_from_ctx(Some("NaN"), Some(90.0)),
        Some(90.0)
    );
    assert_eq!(
        liquidation_mark_from_ctx(Some("bad"), Some(90.0)),
        Some(90.0)
    );
    assert_eq!(liquidation_mark_from_ctx(None, Some(f64::INFINITY)), None);
    assert_eq!(liquidation_mark_from_ctx(None, Some(0.0)), None);
}

#[test]
fn liquidation_request_key_is_stable_for_shared_requests() {
    assert_eq!(
        liquidation_request_key("BTC", 0.0, 161_782.0, 1_778_357_590),
        "BTC:0.00000000:161782.00000000:1778357590"
    );
}

#[test]
fn liquidation_request_coin_reads_shared_request_key() {
    assert_eq!(
        liquidation_request_coin("PURR/USDC:0.00000000:2.00000000:1778357590"),
        "PURR/USDC"
    );
    assert_eq!(liquidation_request_coin("bad-key"), "");
}

#[test]
fn liquidation_plan_waits_when_overlay_is_not_selected() {
    let plan = liquidation_request_plan(LiquidationPlanContext {
        show_liquidations: false,
        liquidation_fetching: false,
        hyperdash_key_missing: true,
        symbol: "BTC",
        ticker_muted: false,
        coin: Some("BTC"),
        mark: Some(100_000.0),
    });

    assert_eq!(plan, LiquidationRequestPlan::Wait);
}

#[test]
fn liquidation_plan_fetches_only_after_overlay_is_selected() {
    let plan = liquidation_request_plan(LiquidationPlanContext {
        show_liquidations: true,
        liquidation_fetching: false,
        hyperdash_key_missing: false,
        symbol: "BTC",
        ticker_muted: false,
        coin: Some("BTC"),
        mark: Some(100_000.0),
    });

    assert_eq!(
        plan,
        LiquidationRequestPlan::Fetch {
            coin: "BTC".to_string(),
            mark: 100_000.0,
        }
    );
}
