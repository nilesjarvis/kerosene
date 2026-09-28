use super::*;

#[test]
fn successful_exchange_results_require_account_refresh() {
    let resting = exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 42_u64
        }
    })]);
    let filled = exchange_response(vec![serde_json::json!({
        "filled": {
            "totalSz": "1",
            "avgPx": "100",
            "oid": 43_u64
        }
    })]);
    let cancel = exchange_response(vec![serde_json::json!("success")]);

    assert!(result_requires_account_refresh(&Ok(resting)));
    assert!(result_requires_account_refresh(&Ok(filled)));
    assert!(result_requires_account_refresh(&Ok(cancel)));
}

#[test]
fn pure_exchange_error_responses_do_not_require_account_refresh() {
    let exchange_error = exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })]);

    assert!(!result_requires_account_refresh(&Ok(exchange_error)));
}

#[test]
fn mixed_exchange_error_responses_require_account_refresh() {
    let later_exchange_error = exchange_response(vec![
        serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }),
        serde_json::json!({
            "error": "Second order rejected"
        }),
    ]);

    assert!(result_requires_account_refresh(&Ok(later_exchange_error)));
}

#[test]
fn ambiguous_transport_results_require_account_refresh() {
    assert!(result_requires_account_refresh(&Err(
        "Exchange request failed: connection closed before response".to_string()
    )));
    assert!(result_requires_account_refresh(&Err(
        "Failed to read response: request body timed out".to_string()
    )));
    assert!(result_requires_account_refresh(&Err(
        "Exchange error: not-json response".to_string()
    )));
}

#[test]
fn execution_result_classifier_normalizes_successful_outcomes() {
    let resting = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "resting": {
            "oid": 42_u64
        }
    })])));
    assert_eq!(resting.kind, ExecutionOutcomeKind::AcceptedResting);
    assert_eq!(resting.status, "Resting (oid 42)");
    assert!(!resting.is_error);
    assert!(resting.refresh_account);

    let filled = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "filled": {
            "totalSz": "1",
            "avgPx": "100",
            "oid": 43_u64
        }
    })])));
    assert_eq!(filled.kind, ExecutionOutcomeKind::Filled);
    assert!(!filled.is_error);
    assert!(filled.refresh_account);

    let filled_without_oid =
        classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100"
            }
        })])));
    assert_eq!(filled_without_oid.kind, ExecutionOutcomeKind::Ambiguous);
    assert_eq!(filled_without_oid.status, "Filled 1 @ $100 (oid ?)");
    assert!(filled_without_oid.is_error);
    assert!(filled_without_oid.refresh_account);

    let cancelled =
        classify_execution_result(Ok(cancel_exchange_response(vec![serde_json::json!(
            "success"
        )])));
    assert_eq!(cancelled.kind, ExecutionOutcomeKind::Cancelled);
    assert_eq!(cancelled.status, "Cancelled");
    assert!(cancelled.refresh_account);

    // A modify/order ack with the same bare-"success" status must not be
    // classified as a cancel; it falls through to Ambiguous, which routes
    // placements into the cloid status check instead of reporting a cancel.
    let acknowledged =
        classify_execution_result(Ok(exchange_response(vec![serde_json::json!("success")])));
    assert_ne!(acknowledged.kind, ExecutionOutcomeKind::Cancelled);
    assert_eq!(acknowledged.status, "Success");
}

#[test]
fn execution_result_classifier_separates_rejected_ambiguous_and_transport_unknown() {
    let rejected = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })])));
    assert_eq!(rejected.kind, ExecutionOutcomeKind::Rejected);
    assert!(rejected.is_error);
    assert!(!rejected.refresh_account);

    let mixed = classify_execution_result(Ok(exchange_response(vec![
        serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }),
        serde_json::json!({
            "error": "Second order rejected"
        }),
    ])));
    assert_eq!(mixed.kind, ExecutionOutcomeKind::Ambiguous);
    assert!(mixed.status.contains("Resting (oid 42)"));
    assert!(mixed.status.contains("Error: Second order rejected"));
    assert!(mixed.is_error);
    assert!(mixed.refresh_account);

    let ambiguous = classify_execution_result(Ok(malformed_ok_response()));
    assert_eq!(ambiguous.kind, ExecutionOutcomeKind::Ambiguous);
    assert_eq!(ambiguous.status, "No response body");
    assert!(ambiguous.is_error);
    assert!(ambiguous.refresh_account);

    let unknown = classify_execution_result(Err(
        "Exchange request failed: connection closed before response".to_string(),
    ));
    assert_eq!(unknown.kind, ExecutionOutcomeKind::TransportUnknown);
    assert!(unknown.is_error);
    assert!(unknown.refresh_account);
}

#[test]
fn execution_result_classifier_redacts_sensitive_external_errors() {
    let rejected = classify_execution_result(Ok(exchange_response(vec![serde_json::json!({
        "error": "Order rejected private_key=super-secret"
    })])));
    assert_eq!(rejected.kind, ExecutionOutcomeKind::Rejected);
    assert!(rejected.status.contains("Order rejected"));
    assert!(rejected.status.contains("private_key=<redacted>"));
    assert!(!rejected.status.contains("super-secret"));

    let unknown = classify_execution_result(Err(
        "Exchange request failed: token=transport-secret".to_string()
    ));
    assert_eq!(unknown.kind, ExecutionOutcomeKind::TransportUnknown);
    assert!(unknown.status.contains("Exchange request failed"));
    assert!(unknown.status.contains("token=<redacted>"));
    assert!(!unknown.status.contains("transport-secret"));
}
