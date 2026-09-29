use super::{exchange_response, exchange_response_from_value, exchange_response_with_statuses};

#[test]
fn ioc_no_match_search_preserves_envelope_raw_and_status_message_rules() {
    let phrase = "ORDER COULD NOT IMMEDIATELY MATCH AGAINST ANY RESTING ORDERS";
    for (envelope, expected) in [
        (serde_json::json!({"status": phrase}), true),
        (
            serde_json::json!({"status": "err", "response": phrase}),
            true,
        ),
        (
            serde_json::json!({"status": "ok", "response": {"message": phrase}}),
            true,
        ),
        (
            serde_json::json!({"status": "ok", "response": format!("api_key=\"{phrase}\"")}),
            false,
        ),
        (
            serde_json::json!({"status": "ok", "response": {"type": "order", "data": {"statuses": [{"error": "unrelated"}, {"error": phrase}]}}}),
            true,
        ),
        (
            serde_json::json!({"status": "ok", "response": {"type": "order", "data": {"statuses": [{"error": null}, {"error": {"message": phrase}}, {"note": phrase}, phrase]}}}),
            false,
        ),
        (
            serde_json::json!({"status": "ok", "response": {"type": "order", "data": {"statuses": []}}}),
            false,
        ),
        (
            serde_json::json!({"status": "ok", "response": {"type": "default"}}),
            false,
        ),
    ] {
        let response = exchange_response_from_value(envelope, "IOC search fixture");
        assert_eq!(response.is_ioc_no_match(), expected);
    }
}

#[test]
fn reported_fills_preserve_size_validation_and_conflicting_status_rules() {
    for size in [
        serde_json::Value::Null,
        serde_json::json!(1),
        serde_json::json!("NaN"),
        serde_json::json!("inf"),
        serde_json::json!("0"),
        serde_json::json!("-1"),
    ] {
        let response =
            exchange_response(serde_json::json!({"filled": {"oid": 42, "totalSz": size}}));
        assert!(response.reports_filled());
        assert!(!response.is_fully_filled());
        assert!(response.is_ambiguous_order_result());
        assert_eq!(response.filled_total_size(), None);
    }
    let missing_size = exchange_response(serde_json::json!({"filled": {"oid": 42}}));
    assert!(missing_size.reports_filled());
    assert!(!missing_size.is_fully_filled());

    for conflict in ["error", "resting"] {
        let mut status = serde_json::json!({"filled": {"oid": 42, "totalSz": "1.25"}});
        status[conflict] = serde_json::Value::Null;
        let response = exchange_response(status);
        assert!(!response.reports_filled());
        assert!(!response.is_fully_filled());
        assert_eq!(response.filled_total_size(), Some(1.25));
    }

    let empty = exchange_response_with_statuses(Vec::new());
    assert!(!empty.reports_filled());
    assert!(!empty.is_fully_filled());
    assert_eq!(empty.filled_total_size(), None);
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
    assert!(response.is_confirmed_modify_result());
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
    assert_eq!(response.filled_total_size(), Some(1.25));
    assert!(!response.is_error());
    assert!(response.is_fully_filled());
    assert!(!response.is_ambiguous_order_result());
    assert!(response.is_confirmed_modify_result());
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
    assert!(!response.has_potential_order_effect());
    assert!(!response.is_fully_filled());
    assert!(!response.is_ambiguous_order_result());
    assert!(!response.is_confirmed_modify_result());
}

#[test]
fn exchange_response_error_status_redacts_sensitive_values() {
    let response = exchange_response(serde_json::json!({
        "error": "rejected api_key=\"exchange-secret\" Authorization: Bearer bearer-secret txid=0x0123456789abcdef0123456789abcdef01234567"
    }));

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

#[test]
fn exchange_response_unknown_status_redacts_sensitive_fallback() {
    let response = exchange_response(serde_json::json!({
        "status": {"api_key": "exchange-secret", "trace": "0x0123456789abcdef0123456789abcdef01234567"}
    }));

    let summary = response.summary();

    assert!(summary.contains("<redacted>"));
    assert!(summary.contains("<redacted-hex>"));
    assert!(!summary.contains("exchange-secret"));
    assert!(!summary.contains("0123456789abcdef0123456789abcdef01234567"));
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
    assert!(response.has_potential_order_effect());
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
    assert_eq!(all_filled.filled_total_size(), Some(3.0));
    assert!(!mixed.is_fully_filled());
    assert_eq!(mixed.filled_total_size(), Some(1.0));
}
