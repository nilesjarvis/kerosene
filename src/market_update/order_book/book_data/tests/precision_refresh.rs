use super::*;

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
