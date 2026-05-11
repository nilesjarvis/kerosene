use super::actions::{build_cancel_action, build_modify_action, build_order_action};
use super::crypto::action_hash_bytes;
use super::{ChaseOrder, ExchangeResponse, OrderKind};

fn exchange_response(status: serde_json::Value) -> ExchangeResponse {
    exchange_response_with_statuses(vec![status])
}

fn exchange_response_with_statuses(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
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
fn action_hash_rejects_invalid_vault_hex() {
    let result = action_hash_bytes(b"{}", Some("0xnot-hex"), 1);

    assert!(result.is_err());
}

#[test]
fn action_hash_rejects_invalid_vault_length() {
    let result = action_hash_bytes(b"{}", Some("0x1234"), 1);

    assert!(result.is_err());
}

#[test]
fn action_hash_accepts_valid_vault_address() {
    let result = action_hash_bytes(b"{}", Some("0x0000000000000000000000000000000000000000"), 1);

    assert!(result.is_ok());
}

#[test]
fn build_order_action_serializes_limit_payload_for_exchange() {
    let action = build_order_action(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        OrderKind::Limit,
        false,
    );
    let json = serde_json::to_value(action).expect("order action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "order",
            "orders": [{
                "a": 7,
                "b": true,
                "p": "123.45",
                "s": "0.25",
                "r": false,
                "t": {
                    "limit": {
                        "tif": "Gtc"
                    }
                }
            }],
            "grouping": "na"
        })
    );
}

#[test]
fn build_order_action_uses_ioc_for_market_and_limit_ioc_and_gtc_for_chase() {
    let market = build_order_action(
        1,
        false,
        "100".to_string(),
        "2".to_string(),
        OrderKind::Market,
        true,
    );
    let limit_ioc = build_order_action(
        1,
        true,
        "101".to_string(),
        "2".to_string(),
        OrderKind::LimitIoc,
        false,
    );
    let chase = build_order_action(
        1,
        true,
        "99".to_string(),
        "2".to_string(),
        OrderKind::Chase,
        false,
    );

    let market_json = serde_json::to_value(market).expect("market action should serialize");
    let limit_ioc_json =
        serde_json::to_value(limit_ioc).expect("limit IOC action should serialize");
    let chase_json = serde_json::to_value(chase).expect("chase action should serialize");

    assert_eq!(market_json["orders"][0]["t"]["limit"]["tif"], "Ioc");
    assert_eq!(market_json["orders"][0]["r"], true);
    assert_eq!(limit_ioc_json["orders"][0]["t"]["limit"]["tif"], "Ioc");
    assert_eq!(chase_json["orders"][0]["t"]["limit"]["tif"], "Gtc");
}

#[test]
fn build_cancel_action_serializes_exchange_payload() {
    let action = build_cancel_action(3, 9001);
    let json = serde_json::to_value(action).expect("cancel action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "cancel",
            "cancels": [{
                "a": 3,
                "o": 9001
            }]
        })
    );
}

#[test]
fn build_modify_action_serializes_exchange_payload() {
    let action = build_modify_action(
        9001,
        3,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        false,
    );
    let json = serde_json::to_value(action).expect("modify action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "batchModify",
            "modifies": [{
                "oid": 9001,
                "order": {
                    "a": 3,
                    "b": true,
                    "p": "123.45",
                    "s": "0.25",
                    "r": false,
                    "t": {
                        "limit": {
                            "tif": "Gtc"
                        }
                    }
                }
            }]
        })
    );
}

#[test]
fn exchange_response_resting_status_reports_oid_without_error() {
    let response = exchange_response(serde_json::json!({
        "resting": {
            "oid": 42_u64
        }
    }));

    assert_eq!(response.summary(), "Resting (oid 42)");
    assert_eq!(response.order_oid(), Some(42));
    assert!(!response.is_error());
    assert!(!response.is_fully_filled());
    assert!(!response.is_ambiguous_order_result());
}

#[test]
fn exchange_response_filled_status_reports_fill_and_completion() {
    let response = exchange_response(serde_json::json!({
        "filled": {
            "totalSz": "1.25",
            "avgPx": "2500.5",
            "oid": 77_u64
        }
    }));

    assert_eq!(response.summary(), "Filled 1.25 @ $2500.5 (oid 77)");
    assert_eq!(response.order_oid(), Some(77));
    assert!(!response.is_error());
    assert!(response.is_fully_filled());
    assert!(!response.is_ambiguous_order_result());
}

#[test]
fn exchange_response_error_status_drives_error_transition() {
    let response = exchange_response(serde_json::json!({
        "error": "Order must have minimum value of $10"
    }));

    assert_eq!(
        response.summary(),
        "Error: Order must have minimum value of $10"
    );
    assert_eq!(response.order_oid(), None);
    assert!(response.is_error());
    assert!(!response.is_fully_filled());
    assert!(!response.is_ambiguous_order_result());
}

#[test]
fn exchange_response_identifies_ioc_no_match_error() {
    let response = exchange_response(serde_json::json!({
        "error": "Order could not immediately match against any resting orders"
    }));

    assert!(response.is_error());
    assert!(response.is_ioc_no_match());

    let other = exchange_response(serde_json::json!({
        "error": "Order must have minimum value of $10"
    }));
    assert!(!other.is_ioc_no_match());
}

#[test]
fn exchange_response_later_error_status_drives_error_transition() {
    let response = exchange_response_with_statuses(vec![
        serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }),
        serde_json::json!({
            "error": "Second order rejected"
        }),
    ]);

    assert_eq!(
        response.summary(),
        "Resting (oid 42); Error: Second order rejected"
    );
    assert!(response.is_error());
    assert!(!response.is_fully_filled());
}

#[test]
fn exchange_response_multiple_filled_statuses_are_all_required_for_completion() {
    let all_filled = exchange_response_with_statuses(vec![
        serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 11_u64
            }
        }),
        serde_json::json!({
            "filled": {
                "totalSz": "2",
                "avgPx": "101",
                "oid": 12_u64
            }
        }),
    ]);
    let mixed = exchange_response_with_statuses(vec![
        serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 11_u64
            }
        }),
        serde_json::json!({
            "resting": {
                "oid": 12_u64
            }
        }),
    ]);

    assert!(all_filled.is_fully_filled());
    assert!(!mixed.is_fully_filled());
}

#[test]
fn exchange_response_ambiguous_ok_body_requires_reconciliation() {
    let malformed: ExchangeResponse = serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": "schema-shifted"
            }
        }
    }))
    .expect("malformed ok-shaped response should preserve the raw body");

    assert_eq!(malformed.summary(), "No response body");
    assert!(!malformed.is_error());
    assert!(malformed.is_ambiguous_order_result());

    let missing_resting_oid = exchange_response(serde_json::json!({
        "resting": {}
    }));
    assert!(!missing_resting_oid.is_error());
    assert!(missing_resting_oid.is_ambiguous_order_result());

    let empty_statuses = exchange_response_with_statuses(Vec::new());
    assert!(!empty_statuses.is_error());
    assert!(empty_statuses.is_ambiguous_order_result());
}

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
    let response: ExchangeResponse = serde_json::from_value(serde_json::json!({
        "status": "err",
        "response": "Failed to deserialize the JSON body into the target type"
    }))
    .expect("error response string should deserialize");

    assert_eq!(
        response.summary(),
        "Error: Failed to deserialize the JSON body into the target type"
    );
    assert!(response.is_error());
    assert_eq!(response.order_oid(), None);
}

#[test]
fn chase_order_debug_redacts_agent_key() {
    let chase = ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "super-secret-agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        remaining_size: 1.0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: std::time::Instant::now(),
        started_at_ms: 1_000,
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: true,
        missing_open_order_refresh_requested: false,
    };

    let rendered = format!("{chase:?}");

    assert!(!rendered.contains("super-secret-agent-key"));
    assert!(rendered.contains("<redacted>"));
}

#[test]
fn chase_price_moves_only_toward_fill() {
    let mut chase = ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        remaining_size: 1.0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: std::time::Instant::now(),
        started_at_ms: 1_000,
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: true,
        missing_open_order_refresh_requested: false,
    };

    assert!(chase.price_moves_toward_fill(100.1));
    assert!(!chase.price_moves_toward_fill(99.9));

    chase.is_buy = false;
    assert!(chase.price_moves_toward_fill(99.9));
    assert!(!chase.price_moves_toward_fill(100.1));
}
