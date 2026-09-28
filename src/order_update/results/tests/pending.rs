use super::*;

#[test]
fn pending_one_shot_status_request_debug_redacts_account_address() {
    let request = PendingOneShotStatusRequest::new(7, &one_shot_context());

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(rendered.contains("0x00000000000000000000000000000000"));
}

#[test]
fn pending_cancel_status_request_debug_redacts_account_and_oid() {
    let request = PendingCancelStatusRequest::new(TEST_ACCOUNT.to_string(), 42, "BTC".to_string());

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(!rendered.contains("42"));
    assert!(rendered.contains("BTC"));
}

#[test]
fn pending_move_status_request_debug_redacts_account_and_oid() {
    let request = PendingMoveStatusRequest::new(TEST_ACCOUNT.to_string(), 42, "BTC".to_string());

    let rendered = format!("{request:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(TEST_ACCOUNT));
    assert!(!rendered.contains("42"));
    assert!(rendered.contains("BTC"));
}
