use super::{exchange_response, exchange_response_with_statuses};

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
    assert_eq!(response.filled_total_size(), Some(1.25));
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
    assert_eq!(all_filled.filled_total_size(), Some(3.0));
    assert!(!mixed.is_fully_filled());
    assert_eq!(mixed.filled_total_size(), Some(1.0));
}
