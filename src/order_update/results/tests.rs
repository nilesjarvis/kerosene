use super::*;

fn exchange_response(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": statuses
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

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
fn failed_exchange_or_transport_results_do_not_require_account_refresh() {
    let exchange_error = exchange_response(vec![serde_json::json!({
        "error": "Order rejected"
    })]);
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

    assert!(!result_requires_account_refresh(&Ok(exchange_error)));
    assert!(!result_requires_account_refresh(&Ok(later_exchange_error)));
    assert!(!result_requires_account_refresh(&Err(
        "network down".to_string()
    )));
}
