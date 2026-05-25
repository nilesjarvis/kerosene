use super::ExchangeResponse;

mod actions;
mod chase;
mod crypto;
mod responses;

fn exchange_response(status: serde_json::Value) -> ExchangeResponse {
    exchange_response_with_statuses(vec![status])
}

fn exchange_response_with_statuses(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": statuses
                }
            }
        }),
        "test exchange response should deserialize",
    )
}

fn exchange_response_from_value(value: serde_json::Value, context: &str) -> ExchangeResponse {
    match serde_json::from_value(value) {
        Ok(response) => response,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn json_value<T: serde::Serialize>(value: T, context: &str) -> serde_json::Value {
    match serde_json::to_value(value) {
        Ok(json) => json,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn msgpack_named<T: serde::Serialize>(value: &T, context: &str) -> Vec<u8> {
    match rmp_serde::to_vec_named(value) {
        Ok(bytes) => bytes,
        Err(error) => panic!("{context}: {error}"),
    }
}
