use super::{exchange_response, exchange_response_from_value, exchange_response_with_statuses};

#[test]
fn exchange_response_ambiguous_ok_body_requires_reconciliation() {
    let malformed = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": "schema-shifted"
                }
            }
        }),
        "malformed ok-shaped response should preserve the raw body",
    );

    assert_eq!(malformed.summary(), "No response body");
    assert!(!malformed.is_error());
    assert!(malformed.is_ambiguous_order_result());

    let missing_resting_oid = exchange_response(serde_json::json!({
        "resting": {}
    }));
    assert!(!missing_resting_oid.is_error());
    assert!(missing_resting_oid.is_ambiguous_order_result());

    let invalid_filled_size = exchange_response(serde_json::json!({
        "filled": {
            "totalSz": "NaN",
            "avgPx": "100",
            "oid": 42_u64
        }
    }));
    assert_eq!(invalid_filled_size.filled_total_size(), None);
    assert!(invalid_filled_size.is_ambiguous_order_result());

    let empty_statuses = exchange_response_with_statuses(Vec::new());
    assert!(!empty_statuses.is_error());
    assert!(empty_statuses.is_ambiguous_order_result());
}
