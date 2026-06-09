use super::*;

#[test]
fn precision_refresh_waits_until_mid_is_known() {
    assert!(!order_book_needs_precision_refresh(
        50.0, None, None, None, false, None
    ));
}

#[test]
fn precision_refresh_requests_saved_coarse_tick_after_mid_arrives() {
    assert!(order_book_needs_precision_refresh(
        50.0,
        None,
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
        Some(mid),
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
        None,
        false,
        Some(80_000.0)
    ));
}

#[test]
fn precision_refresh_requests_matching_source_depth_after_upward_scope_drift() {
    let source_mid = 80_000.0;
    let current_mid = 81_000.0;
    let selected_tick = 100.0;
    let source_tick = helpers::sigfig_server_tick(
        helpers::compute_sigfigs(selected_tick, source_mid),
        source_mid,
    );

    assert!(order_book_needs_precision_refresh(
        selected_tick,
        source_tick,
        Some(source_mid),
        None,
        false,
        Some(current_mid)
    ));
}

#[test]
fn precision_refresh_requests_matching_source_depth_after_downward_scope_drift() {
    let source_mid = 80_000.0;
    let current_mid = 79_000.0;
    let selected_tick = 100.0;
    let source_tick = helpers::sigfig_server_tick(
        helpers::compute_sigfigs(selected_tick, source_mid),
        source_mid,
    );

    assert!(order_book_needs_precision_refresh(
        selected_tick,
        source_tick,
        Some(source_mid),
        None,
        false,
        Some(current_mid)
    ));
}

#[test]
fn precision_refresh_keeps_matching_source_depth_when_scope_is_current() {
    let source_mid = 80_000.0;
    let current_mid = 80_999.0;
    let selected_tick = 100.0;
    let source_tick = helpers::sigfig_server_tick(
        helpers::compute_sigfigs(selected_tick, source_mid),
        source_mid,
    );

    assert!(!order_book_needs_precision_refresh(
        selected_tick,
        source_tick,
        Some(source_mid),
        None,
        false,
        Some(current_mid)
    ));
}

#[test]
fn precision_refresh_ids_include_books_with_stale_source_scope() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.all_mids.insert("BTC".to_string(), 81_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let mut instance =
        OrderBookInstance::new(42, OrderBookSymbolMode::Fixed("BTC".to_string()), 100.0);
    instance.set_book_with_source(
        OrderBook {
            bids: vec![crate::api::BookLevel {
                px: 79_900.0,
                sz: 1.0,
            }],
            asks: vec![crate::api::BookLevel {
                px: 80_100.0,
                sz: 1.0,
            }],
        },
        Some(100.0),
    );
    terminal.order_books.insert(42, instance);

    assert_eq!(terminal.order_book_precision_refresh_ids(), vec![42]);
}
