use super::super::{moved_order_is_buy, moved_order_reduce_only, moved_order_size_wire};
use super::fixtures::reduce_only_error_or_panic;
use crate::api::MarketType;

#[test]
fn moved_order_size_returns_canonical_wire_size() {
    assert_eq!(moved_order_size_wire(" 0.25 "), Some("0.25".to_string()));
}

#[test]
fn moved_order_size_rejects_invalid_values() {
    assert_eq!(moved_order_size_wire(""), None);
    assert_eq!(moved_order_size_wire("abc"), None);
    assert_eq!(moved_order_size_wire("0"), None);
    assert_eq!(moved_order_size_wire("-1"), None);
    assert_eq!(moved_order_size_wire("NaN"), None);
    assert_eq!(moved_order_size_wire("inf"), None);
}

#[test]
fn moved_order_side_accepts_only_exchange_bid_or_ask_markers() {
    assert_eq!(moved_order_is_buy("B"), Some(true));
    assert_eq!(moved_order_is_buy("A"), Some(false));
    assert_eq!(moved_order_is_buy("buy"), None);
    assert_eq!(moved_order_is_buy(""), None);
}

#[test]
fn moved_order_reduce_only_preserves_known_perp_metadata() {
    assert_eq!(
        moved_order_reduce_only(MarketType::Perp, Some(true)),
        Ok(true)
    );
    assert_eq!(
        moved_order_reduce_only(MarketType::Perp, Some(false)),
        Ok(false)
    );
}

#[test]
fn moved_order_reduce_only_rejects_unknown_perp_metadata() {
    assert!(
        reduce_only_error_or_panic(moved_order_reduce_only(MarketType::Perp, None))
            .contains("reduce-only metadata is unavailable")
    );
}

#[test]
fn moved_order_reduce_only_ignores_missing_spot_metadata() {
    assert_eq!(moved_order_reduce_only(MarketType::Spot, None), Ok(false));
}

#[test]
fn moved_order_reduce_only_ignores_missing_outcome_metadata() {
    assert_eq!(
        moved_order_reduce_only(MarketType::Outcome, None),
        Ok(false)
    );
}
