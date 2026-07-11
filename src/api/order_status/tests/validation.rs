use super::*;

#[test]
fn order_status_error_preview_redacts_sensitive_values() {
    let preview = order_status_error_preview(
        "upstream echoed Authorization: Token token-secret refreshToken=\"refresh-secret\" user=0xabc0000000000000000000000000000000000000",
    );

    assert!(preview.contains("Authorization: Token <redacted>"));
    assert!(preview.contains("<redacted-hex>"));
    for secret in [
        "token-secret",
        "refresh-secret",
        "abc0000000000000000000000000000000000000",
    ] {
        assert!(
            !preview.contains(secret),
            "orderStatus preview leaked {secret}"
        );
    }
}

#[test]
fn order_status_error_preview_redacts_cloid_before_truncation() {
    let cloid = "0x1234567890abcdef1234567890abcdef";
    let body = format!("{} {cloid}", "x".repeat(130));

    let preview = order_status_error_preview(&body);

    assert!(preview.contains("<redacted-hex>"));
    assert!(!preview.contains("0x1234567890abcdef"));
    assert!(!preview.contains(cloid));
}

#[test]
fn order_status_result_error_is_redacted_before_message_mapping() {
    let cloid = "0x1234567890abcdef1234567890abcdef";
    let result = redact_order_status_result(Err(format!(
        "orderStatus failed: api_key=status-secret cloid={cloid}"
    )));
    let debug = format!("{result:?}");
    let error = result.expect_err("error should remain an error");

    assert!(error.contains("orderStatus failed"));
    assert!(error.contains("<redacted>"));
    assert!(error.contains("<redacted-hex>"));
    for secret in ["status-secret", cloid] {
        assert!(!error.contains(secret), "status error leaked {secret}");
        assert!(
            !debug.contains(secret),
            "status result debug leaked {secret}"
        );
    }
}

#[test]
fn order_status_result_redaction_preserves_success_and_safe_error_text() {
    let status = status_or_panic(&serde_json::json!({"status": "unknownOid"}));
    let status = redact_order_status_result(Ok(status)).expect("success should remain successful");
    assert!(status.is_missing());

    let safe = "orderStatus request failed: connection closed";
    let error = redact_order_status_result(Err(safe.to_string()))
        .expect_err("safe error should remain an error");
    assert_eq!(error, safe);
}

#[test]
fn rejects_mismatched_order_status_cloid() {
    let error = cloid_status_error_or_panic(
        &serde_json::json!({
            "status": "order",
            "order": {
                "status": "open",
                "order": {
                    "oid": 42_u64,
                    "cloid": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                }
            }
        }),
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    assert!(error.contains("cloid mismatch"));
    assert!(!error.contains("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    assert!(!error.contains("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
}

#[test]
fn rejects_order_status_without_expected_cloid() {
    let error = cloid_status_error_or_panic(
        &serde_json::json!({
            "status": "order",
            "order": {
                "status": "open",
                "order": {
                    "oid": 42_u64
                }
            }
        }),
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    assert!(error.contains("missing cloid"));
    assert!(!error.contains("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
}

#[test]
fn rejects_mismatched_order_status_oid() {
    let error = oid_status_error_or_panic(
        &serde_json::json!({
            "status": "order",
            "order": {
                "status": "open",
                "order": {
                    "oid": 43_u64
                }
            }
        }),
        42,
    );

    assert!(error.contains("oid mismatch"));
    assert!(!error.contains("42"));
    assert!(!error.contains("43"));
}

#[test]
fn rejects_order_status_without_expected_oid() {
    let error = oid_status_error_or_panic(
        &serde_json::json!({
            "status": "order",
            "order": {
                "status": "open",
                "order": {
                    "cloid": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }
            }
        }),
        42,
    );

    assert!(error.contains("missing oid"));
    assert!(!error.contains("42"));
}
