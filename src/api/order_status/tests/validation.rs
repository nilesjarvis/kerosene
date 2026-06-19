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
    assert!(error.contains("got 43"));
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
}
