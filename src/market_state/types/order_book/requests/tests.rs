use super::*;
use crate::market_state::OrderBookSymbolMode;

#[test]
fn pending_requests_preserve_exact_identity_fuzzy_ticks_and_wrapping_ids() {
    let mut inst = OrderBookInstance::new(7, OrderBookSymbolMode::Active, 100.0);
    let precision = (Some(5), Some(2));
    assert!(!inst.pending_book_request_matches("BTC", 100.0, precision));
    let first = inst.mark_book_request("BTC".to_string(), 100.0, precision);
    assert_eq!(first, 1);
    assert_eq!(inst.pending_book_sigfigs(), Some(precision));
    for (symbol, tick, sigfigs, expected) in [
        ("BTC", 100.0, precision, true),
        ("BTC", 101.0, precision, true),
        ("BTC", 102.0, precision, false),
        ("btc", 100.0, precision, false),
        ("BTC ", 100.0, precision, false),
        ("BTC", 100.0, (Some(5), None), false),
        ("BTC", 0.0, precision, false),
        ("BTC", f64::NAN, precision, false),
        ("BTC", f64::INFINITY, precision, false),
    ] {
        assert_eq!(
            inst.pending_book_request_matches(symbol, tick, sigfigs),
            expected
        );
        assert_eq!(
            inst.pending_book_request_matches_id(first, symbol, tick, sigfigs),
            expected
        );
        assert!(!inst.pending_book_request_matches_id(first + 1, symbol, tick, sigfigs));
        inst.clear_matching_book_request(first + 1, symbol, tick, sigfigs);
        assert_eq!(inst.pending_book_request_id(), Some(first));
        if !expected {
            inst.clear_matching_book_request(first, symbol, tick, sigfigs);
            assert_eq!(inst.pending_book_request_id(), Some(first));
        }
    }
    inst.clear_matching_book_request(first, "BTC", 101.0, precision);
    assert_eq!(inst.pending_book_request_id(), None);
    assert_eq!(inst.pending_book_sigfigs(), None);
    let second = inst.mark_book_request("ETH".to_string(), 1.0, (None, None));
    assert_eq!(second, 2);
    inst.next_book_request_id = u64::MAX;
    let wrapped = inst.mark_book_request("BTC".to_string(), 100.0, precision);
    assert_eq!(wrapped, 0);
    assert!(inst.pending_book_request_matches_id(wrapped, "BTC", 100.0, precision));
    inst.clear_matching_book_request(second, "ETH", 1.0, (None, None));
    assert_eq!(inst.pending_book_request_id(), Some(wrapped));
    inst.clear_book_request();
    assert_eq!(inst.pending_book_request_id(), None);
}
