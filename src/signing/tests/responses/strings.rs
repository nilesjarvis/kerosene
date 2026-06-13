use super::{exchange_response, exchange_response_from_value};

#[test]
fn exchange_response_success_string_reports_cancelled_only_for_cancel_actions() {
    let cancel_response = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "cancel",
                "data": {
                    "statuses": ["success"]
                }
            }
        }),
        "cancel response should deserialize",
    );

    assert_eq!(cancel_response.summary(), "Cancelled");
    assert_eq!(cancel_response.order_oid(), None);
    assert!(!cancel_response.is_error());
    assert!(!cancel_response.is_fully_filled());
    assert!(cancel_response.is_confirmed_cancel_result());

    // A modify/order ack of the same shape is a generic acknowledgement; it
    // must not be reported as a cancel.
    let order_response = exchange_response(serde_json::json!("success"));

    assert_eq!(order_response.summary(), "Success");
    assert_eq!(order_response.order_oid(), None);
    assert!(!order_response.is_error());
    assert!(!order_response.is_fully_filled());
    assert!(!order_response.is_confirmed_cancel_result());
    assert!(order_response.is_confirmed_modify_result());
}

#[test]
fn exchange_response_requires_explicit_cancel_success_for_confirmed_cancel() {
    let empty_cancel_response = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "cancel",
                "data": {
                    "statuses": []
                }
            }
        }),
        "empty cancel response should deserialize",
    );

    assert_eq!(empty_cancel_response.summary(), "OK (no statuses)");
    assert!(!empty_cancel_response.is_error());
    assert!(!empty_cancel_response.is_confirmed_cancel_result());
}

#[test]
fn exchange_response_requires_default_body_for_confirmed_default_result() {
    let default_response = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "default"
            }
        }),
        "default response should deserialize",
    );
    let missing_body = exchange_response_from_value(
        serde_json::json!({
            "status": "ok"
        }),
        "missing-body response should deserialize",
    );
    let raw_body = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": "schema-shifted"
        }),
        "raw response should deserialize",
    );
    let order_response = exchange_response(serde_json::json!("success"));

    assert!(default_response.is_confirmed_default_result());
    assert!(!missing_body.is_confirmed_default_result());
    assert!(!raw_body.is_confirmed_default_result());
    assert!(!order_response.is_confirmed_default_result());
}

#[test]
fn exchange_response_error_string_body_reports_exchange_error() {
    let response = exchange_response_from_value(
        serde_json::json!({
            "status": "err",
            "response": "Failed to deserialize the JSON body into the target type"
        }),
        "error response string should deserialize",
    );

    assert_eq!(
        response.summary(),
        "Error: Failed to deserialize the JSON body into the target type"
    );
    assert!(response.is_error());
    assert_eq!(response.order_oid(), None);
}

#[test]
fn exchange_response_error_string_body_redacts_sensitive_values() {
    let response = exchange_response_from_value(
        serde_json::json!({
            "status": "err",
            "response": "upstream echoed api_key=\"exchange-secret\" Authorization: Bearer bearer-secret sig=0x0123456789abcdef0123456789abcdef01234567"
        }),
        "error response string should deserialize",
    );

    let summary = response.summary();

    assert!(summary.contains("<redacted>"));
    assert!(summary.contains("<redacted-hex>"));
    for secret in [
        "exchange-secret",
        "bearer-secret",
        "0123456789abcdef0123456789abcdef01234567",
    ] {
        assert!(!summary.contains(secret), "summary leaked {secret}");
    }
}
