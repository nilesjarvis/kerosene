use crate::api::OrderStatusResult;
use crate::signing::ExchangeResponse;

pub(in crate::order_execution::twap::tests) fn exchange_response_from_value(
    value: serde_json::Value,
    context: &str,
) -> ExchangeResponse {
    match serde_json::from_value(value) {
        Ok(response) => response,
        Err(error) => panic!("{context}: {error}"),
    }
}

pub(in crate::order_execution::twap::tests) fn exchange_response(
    status: serde_json::Value,
) -> ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [status]
                }
            }
        }),
        "test exchange response should deserialize",
    )
}

pub(in crate::order_execution::twap::tests) fn filled_status(
    cloid: &str,
    oid: u64,
) -> OrderStatusResult {
    OrderStatusResult {
        status: "filled".to_string(),
        oid: Some(oid),
        cloid: Some(cloid.to_string()),
        raw_summary: format!("filled oid={oid} cloid={cloid}"),
    }
}

pub(in crate::order_execution::twap::tests) fn missing_status(cloid: &str) -> OrderStatusResult {
    OrderStatusResult {
        status: "unknownOid".to_string(),
        oid: None,
        cloid: Some(cloid.to_string()),
        raw_summary: format!("unknownOid cloid={cloid}"),
    }
}
