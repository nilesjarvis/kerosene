use super::chase_terminal_cancel_error;

#[test]
fn terminal_cancel_error_detects_already_gone_orders() {
    assert!(chase_terminal_cancel_error(
        "Error: Order was never placed, already canceled, or filled"
    ));
    assert!(chase_terminal_cancel_error("cannot cancel cancelled order"));
    assert!(chase_terminal_cancel_error("cannot cancel cancled order"));
    assert!(chase_terminal_cancel_error("order no longer open"));
    assert!(chase_terminal_cancel_error("order not found"));
}

#[test]
fn terminal_cancel_error_rejects_unrelated_cancel_failures() {
    assert!(!chase_terminal_cancel_error("Error: rate limited"));
    assert!(!chase_terminal_cancel_error("Exchange request failed"));
    assert!(!chase_terminal_cancel_error("invalid signature"));
}
