use super::*;

#[test]
fn order_book_fetch_plan_uses_fixed_symbol() {
    let plan = required_plan(
        plan_order_book_fetch(
            7,
            &OrderBookSymbolMode::Fixed("ETH".to_string()),
            "BTC",
            0.1,
            3_500.0,
            None,
            false,
        ),
        "fixed order books should plan a fetch",
    );

    assert_eq!(plan.id, 7);
    assert_eq!(plan.symbol, "ETH");
    assert_eq!(plan.sigfigs, helpers::compute_sigfigs(0.1, 3_500.0));
}

#[test]
fn order_book_fetch_plan_uses_active_symbol() {
    let plan = required_plan(
        plan_order_book_fetch(
            1,
            &OrderBookSymbolMode::Active,
            "BTC",
            1.0,
            80_000.0,
            None,
            false,
        ),
        "active order books should plan a fetch",
    );

    assert_eq!(plan.symbol, "BTC");
}

#[test]
fn order_book_fetch_plan_falls_back_to_live_mid_when_book_is_empty() {
    let plan = required_plan(
        plan_order_book_fetch(
            1,
            &OrderBookSymbolMode::Active,
            "BTC",
            1.0,
            0.0,
            Some(80_000.0),
            false,
        ),
        "live mid should be enough to request aggregated depth",
    );

    assert_eq!(plan.sigfigs, helpers::compute_sigfigs(1.0, 80_000.0));
}

#[test]
fn order_book_fetch_plan_skips_empty_or_muted_symbols() {
    assert!(
        plan_order_book_fetch(
            1,
            &OrderBookSymbolMode::Active,
            "",
            1.0,
            80_000.0,
            None,
            false
        )
        .is_none()
    );
    assert!(
        plan_order_book_fetch(
            1,
            &OrderBookSymbolMode::Fixed("BTC".to_string()),
            "ETH",
            1.0,
            80_000.0,
            None,
            true
        )
        .is_none()
    );
}
