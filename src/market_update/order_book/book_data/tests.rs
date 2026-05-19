use super::*;

#[test]
fn order_book_fetch_plan_uses_fixed_symbol() {
    let plan = plan_order_book_fetch(
        7,
        &OrderBookSymbolMode::Fixed("ETH".to_string()),
        "BTC",
        0.1,
        3_500.0,
        None,
        false,
    )
    .expect("fixed order books should plan a fetch");

    assert_eq!(plan.id, 7);
    assert_eq!(plan.symbol, "ETH");
    assert_eq!(plan.sigfigs, helpers::compute_sigfigs(0.1, 3_500.0));
}

#[test]
fn order_book_fetch_plan_uses_active_symbol() {
    let plan = plan_order_book_fetch(
        1,
        &OrderBookSymbolMode::Active,
        "BTC",
        1.0,
        80_000.0,
        None,
        false,
    )
    .expect("active order books should plan a fetch");

    assert_eq!(plan.symbol, "BTC");
}

#[test]
fn outcome_symbols_are_available_for_order_book_fetches() {
    let terminal = TradingTerminal::boot().0;

    assert_eq!(terminal.order_book_unavailable_reason("#650"), None);
}

#[test]
fn order_book_fetch_plan_falls_back_to_live_mid_when_book_is_empty() {
    let plan = plan_order_book_fetch(
        1,
        &OrderBookSymbolMode::Active,
        "BTC",
        1.0,
        0.0,
        Some(80_000.0),
        false,
    )
    .expect("live mid should be enough to request aggregated depth");

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

#[test]
fn precision_refresh_waits_until_mid_is_known() {
    assert!(!order_book_needs_precision_refresh(
        50.0, None, None, false, None
    ));
}

#[test]
fn precision_refresh_requests_saved_coarse_tick_after_mid_arrives() {
    assert!(order_book_needs_precision_refresh(
        50.0,
        None,
        None,
        false,
        Some(80_000.0)
    ));
}

#[test]
fn precision_refresh_skips_expected_request_already_in_flight() {
    let mid = 80_000.0;
    let expected = helpers::compute_sigfigs(50.0, mid);

    assert!(!order_book_needs_precision_refresh(
        50.0,
        None,
        Some(expected),
        false,
        Some(mid)
    ));
}

#[test]
fn precision_refresh_skips_when_any_book_request_is_in_flight() {
    let pending = helpers::compute_sigfigs(50.0, 80_000.0);

    assert!(!order_book_needs_precision_refresh(
        50.0,
        None,
        Some(pending),
        true,
        Some(100_000.0)
    ));
}

#[test]
fn precision_refresh_skips_matching_source_depth() {
    let mid = 80_000.0;
    let expected_source = helpers::sigfig_server_tick(helpers::compute_sigfigs(50.0, mid), mid);

    assert!(!order_book_needs_precision_refresh(
        50.0,
        expected_source,
        None,
        false,
        Some(mid)
    ));
}

#[test]
fn precision_refresh_does_not_refetch_default_tick() {
    assert!(!order_book_needs_precision_refresh(
        helpers::default_tick_for_price(80_000.0),
        None,
        None,
        false,
        Some(80_000.0)
    ));
}

#[test]
fn stale_precision_response_is_rejected_after_mid_is_known() {
    let mid = 80_000.0;

    assert!(!order_book_response_matches_expected_precision(
        50.0,
        (None, None),
        Some(mid)
    ));
    assert!(order_book_response_matches_expected_precision(
        50.0,
        helpers::compute_sigfigs(50.0, mid),
        Some(mid)
    ));
}

#[test]
fn stale_precision_response_guard_allows_default_tick() {
    let mid = 80_000.0;

    assert!(order_book_response_matches_expected_precision(
        helpers::default_tick_for_price(mid),
        (None, None),
        Some(mid)
    ));
}
