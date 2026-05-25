use super::*;

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
