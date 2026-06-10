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

    // A modify/order ack of the same shape is a generic acknowledgement; it
    // must not be reported as a cancel.
    let order_response = exchange_response(serde_json::json!("success"));

    assert_eq!(order_response.summary(), "Success");
    assert_eq!(order_response.order_oid(), None);
    assert!(!order_response.is_error());
    assert!(!order_response.is_fully_filled());
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
