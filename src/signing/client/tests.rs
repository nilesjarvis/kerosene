use super::super::actions::HyperliquidL1Action;
use super::{
    EXCHANGE_EXPIRES_AFTER_MS, allocate_exchange_nonce_from,
    build_signed_exchange_payload_with_nonce, exchange_payload_action,
    exchange_payload_contains_private_key, exchange_payload_expires_after, exchange_payload_nonce,
    exchange_payload_signature, exchange_payload_vault_address, parse_exchange_response,
};
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

const TEST_PRIVATE_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

#[test]
fn exchange_nonce_allocator_is_monotonic_inside_same_millisecond() {
    let last_nonce = AtomicU64::new(0);

    let first = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
    let second = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);
    let third = allocate_exchange_nonce_from(&last_nonce, 1_700_000_000_000);

    assert_eq!(first, 1_700_000_000_000);
    assert_eq!(second, first + 1);
    assert_eq!(third, second + 1);
}

#[test]
fn exchange_nonce_allocator_never_moves_backwards_when_clock_regresses() {
    let last_nonce = AtomicU64::new(5_000);

    let nonce = allocate_exchange_nonce_from(&last_nonce, 4_000);

    assert_eq!(nonce, 5_001);
    assert_eq!(last_nonce.load(Ordering::SeqCst), 5_001);
}

#[test]
fn signed_exchange_payload_contains_signed_request_fields_without_private_key() {
    let nonce = 1_700_000_000_000;
    let vault_address = "0x0000000000000000000000000000000000000001";
    let action = HyperliquidL1Action::cancel(110_003, 42);

    let payload = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(TEST_PRIVATE_KEY.to_string()),
        &action,
        Some(vault_address),
        nonce,
    )
    .expect("payload should sign");

    assert_eq!(exchange_payload_nonce(&payload), Some(nonce));
    assert_eq!(
        exchange_payload_expires_after(&payload),
        Some(nonce + EXCHANGE_EXPIRES_AFTER_MS)
    );
    assert_eq!(
        exchange_payload_vault_address(&payload),
        Some(vault_address)
    );
    assert!(exchange_payload_action(&payload).is_some());
    let signature = exchange_payload_signature(&payload).expect("signature should be present");
    assert!(
        signature
            .get("r")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert!(
        signature
            .get("s")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert!(
        signature
            .get("v")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    );
    assert!(!exchange_payload_contains_private_key(
        &payload,
        TEST_PRIVATE_KEY
    ));
}

#[test]
fn signed_exchange_payload_error_does_not_echo_private_key() {
    let invalid_key = format!("{TEST_PRIVATE_KEY}ff");
    let action = HyperliquidL1Action::cancel(110_003, 42);

    let error = build_signed_exchange_payload_with_nonce(
        Zeroizing::new(invalid_key.clone()),
        &action,
        None,
        1_700_000_000_000,
    )
    .expect_err("invalid private key should fail before posting");

    assert!(error.contains("Invalid private key hex"));
    assert!(!error.contains(&invalid_key));
    assert!(!error.contains(TEST_PRIVATE_KEY));
}

#[test]
fn parse_exchange_response_accepts_valid_exchange_json() {
    let response = parse_exchange_response(r#"{"status":"ok","response":{"type":"order"}}"#)
        .expect("valid exchange response should parse");

    assert_eq!(response.status, "ok");
    assert_eq!(
        response
            .response
            .as_ref()
            .expect("response body should be parsed")
            .response_type,
        "order"
    );
}

#[test]
fn parse_exchange_response_redacts_sensitive_raw_body_on_error() {
    let signature = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let raw = format!(
        "upstream parse failed Authorization: Bearer exchange-secret api_key=\"json-secret\" txid={signature}"
    );

    let error = parse_exchange_response(&raw).expect_err("malformed body should fail");

    assert!(error.contains("Exchange error:"));
    assert!(error.contains("<redacted>"));
    assert!(error.contains("<redacted-hex>"));
    for secret in [
        "exchange-secret",
        "json-secret",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        assert!(
            !error.contains(secret),
            "exchange parse error leaked {secret}"
        );
    }
}
