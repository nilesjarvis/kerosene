use super::{exchange_response, exchange_response_from_value};

#[test]
fn exchange_response_success_string_reports_cancelled() {
    let response = exchange_response(serde_json::json!("success"));

    assert_eq!(response.summary(), "Cancelled");
    assert_eq!(response.order_oid(), None);
    assert!(!response.is_error());
    assert!(!response.is_fully_filled());
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
