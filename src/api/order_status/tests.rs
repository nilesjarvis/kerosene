use super::{
    OrderStatusResult, order_status_error_preview, parse_order_status,
    parse_order_status_for_cloid, parse_order_status_for_oid,
};

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

fn oid_status_error_or_panic(value: &serde_json::Value, expected_oid: u64) -> String {
    match parse_order_status_for_oid(value, expected_oid) {
        Ok(status) => panic!("expected oid status error, got {status:?}"),
        Err(error) => error,
    }
}

fn status_error_or_panic(value: &serde_json::Value) -> String {
    match parse_order_status(value) {
        Ok(status) => panic!("expected order status error, got {status:?}"),
        Err(error) => error,
    }
}
