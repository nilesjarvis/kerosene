use super::cancel_error_indicates_closed_order;

#[test]
fn terminal_cancel_error_detects_already_gone_orders() {
    assert!(cancel_error_indicates_closed_order(
        "Error: Order was never placed, already canceled, or filled"
    ));
    assert!(cancel_error_indicates_closed_order(
        "cannot cancel cancelled order"
    ));
    assert!(cancel_error_indicates_closed_order(
        "cannot cancel cancled order"
    ));
    assert!(cancel_error_indicates_closed_order("order no longer open"));
    assert!(cancel_error_indicates_closed_order("order not found"));
}

#[test]
fn terminal_cancel_error_rejects_unrelated_cancel_failures() {
    assert!(!cancel_error_indicates_closed_order("Error: rate limited"));
    assert!(!cancel_error_indicates_closed_order(
        "Exchange request failed"
    ));
    assert!(!cancel_error_indicates_closed_order("invalid signature"));
}

#[test]
fn terminal_cancel_error_preserves_ascii_substring_matching() {
    for phrase in [
        "filled",
        "canceled",
        "cancelled",
        "cancled",
        "never placed",
        "not found",
        "does not exist",
        "no open order",
        "no longer open",
    ] {
        for summary in [
            phrase.to_string(),
            phrase.to_ascii_uppercase(),
            format!("prefix{phrase}suffix"),
        ] {
            assert!(cancel_error_indicates_closed_order(&summary), "{summary}");
        }
    }
    for summary in [
        "",
        "invalid signature",
        "rate limited",
        "no  open order",
        "not-found",
        "fİlled",
        "never\nplaced",
    ] {
        assert!(!cancel_error_indicates_closed_order(summary), "{summary}");
    }
}
