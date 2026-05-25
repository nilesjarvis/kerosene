use super::summary::hide_order_oid_references;

#[test]
fn hides_parenthesized_oid_from_history_summary() {
    assert_eq!(
        hide_order_oid_references("Chase filled: BUY 0.3 BTC @ $106 (oid 42)"),
        "Chase filled: BUY 0.3 BTC @ $106"
    );
    assert_eq!(
        hide_order_oid_references("Resting (oid 42); Error: rejected"),
        "Resting; Error: rejected"
    );
}

#[test]
fn hides_inline_oid_from_history_summary() {
    assert_eq!(
        hide_order_oid_references("Slice 2 unexpectedly rested as oid 123; cancelling"),
        "Slice 2 unexpectedly rested as order; cancelling"
    );
    assert_eq!(
        hide_order_oid_references("filled oid=123 cloid=0xabc"),
        "filled order cloid=0xabc"
    );
}
