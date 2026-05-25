use super::*;

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
