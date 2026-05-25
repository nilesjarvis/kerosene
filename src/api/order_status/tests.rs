use super::{OrderStatusResult, parse_order_status, parse_order_status_for_cloid};

mod parsing;
mod validation;

fn status_or_panic(value: &serde_json::Value) -> OrderStatusResult {
    match parse_order_status(value) {
        Ok(status) => status,
        Err(error) => panic!("order status should parse: {error}"),
    }
}

fn cloid_status_error_or_panic(value: &serde_json::Value, expected_cloid: &str) -> String {
    match parse_order_status_for_cloid(value, expected_cloid) {
        Ok(status) => panic!("expected cloid status error, got {status:?}"),
        Err(error) => error,
    }
}
