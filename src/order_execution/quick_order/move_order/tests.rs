use super::{
    moved_order_is_buy, moved_order_price_wire, moved_order_reduce_only, moved_order_size_wire,
};
use crate::api::MarketType;
use crate::order_execution::{MoveOrderContextError, PendingMoveOrderContext};

#[test]
fn moved_order_price_returns_none_when_rounded_price_is_unchanged() {
    assert_eq!(moved_order_price_wire(100.001, 100.0, 2, false), None);
}

#[test]
fn moved_order_price_returns_wire_price_when_rounded_price_changes() {
    assert_eq!(
        moved_order_price_wire(101.0, 100.0, 2, false),
        Some("101".to_string())
    );
}

#[test]
fn moved_order_price_rejects_nonfinite_new_price() {
    assert_eq!(moved_order_price_wire(f64::NAN, 100.0, 2, false), None);
    assert_eq!(moved_order_price_wire(f64::INFINITY, 100.0, 2, false), None);
}

#[test]
fn moved_order_price_rejects_invalid_original_price() {
    assert_eq!(moved_order_price_wire(101.0, f64::NAN, 2, false), None);
    assert_eq!(moved_order_price_wire(101.0, 0.0, 2, false), None);
    assert_eq!(moved_order_price_wire(101.0, -1.0, 2, false), None);
}

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
        moved_order_reduce_only(MarketType::Perp, None)
            .expect_err("unknown reduce-only should be rejected")
            .contains("reduce-only metadata is unavailable")
    );
}

#[test]
fn moved_order_reduce_only_ignores_missing_spot_metadata() {
    assert_eq!(moved_order_reduce_only(MarketType::Spot, None), Ok(false));
}

#[test]
fn pending_move_context_reuses_captured_agent_key_for_same_account() {
    let context = PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        "original-agent-key",
    )
    .expect("valid context");

    assert_eq!(
        context.replacement_agent_key(Some("0xabc0000000000000000000000000000000000000")),
        Ok("original-agent-key".to_string().into())
    );
}

#[test]
fn pending_move_context_rejects_replacement_after_account_change() {
    let context = PendingMoveOrderContext::new(
        "0xabc0000000000000000000000000000000000000",
        "original-agent-key",
    )
    .expect("valid context");

    assert_eq!(
        context.replacement_agent_key(Some("0xdef0000000000000000000000000000000000000")),
        Err(MoveOrderContextError::AccountChanged)
    );
    assert_eq!(
        context.replacement_agent_key(None),
        Err(MoveOrderContextError::AccountChanged)
    );
}

#[test]
fn pending_move_context_rejects_empty_agent_key() {
    assert!(matches!(
        PendingMoveOrderContext::new("0xabc0000000000000000000000000000000000000", "   "),
        Err(MoveOrderContextError::MissingAgentKey)
    ));
}
