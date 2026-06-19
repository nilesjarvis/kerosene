use super::*;

#[test]
fn parses_order_status_by_cloid_response() {
    let parsed = status_or_panic(&serde_json::json!({
        "status": "order",
        "order": {
            "status": "open",
            "order": {
                "oid": 42_u64,
                "cloid": "0x1234567890abcdef1234567890abcdef"
            }
        }
    }));

    assert!(parsed.is_open());
    assert_eq!(parsed.oid, Some(42));
    assert_eq!(
        parsed.cloid.as_deref(),
        Some("0x1234567890abcdef1234567890abcdef")
    );
}

#[test]
fn order_status_result_debug_redacts_identifiers_and_raw_summary() {
    let parsed = status_or_panic(&serde_json::json!({
        "status": "order",
        "order": {
            "status": "open",
            "order": {
                "oid": 424242_u64,
                "cloid": "0x1234567890abcdef1234567890abcdef"
            }
        }
    }));

    let rendered = format!("{parsed:?}");

    assert!(rendered.contains("OrderStatusResult"));
    assert!(rendered.contains("status: \"open\""));
    assert!(rendered.contains("has_oid: true"));
    assert!(rendered.contains("has_cloid: true"));
    assert!(rendered.contains("raw_summary: \"<redacted>\""));
    assert!(!rendered.contains("424242"));
    assert!(!rendered.contains("0x1234567890abcdef1234567890abcdef"));
}

#[test]
fn parses_missing_order_status() {
    let parsed = status_or_panic(&serde_json::json!({
        "status": "unknownOid"
    }));

    assert!(parsed.is_missing());
}

#[test]
fn parsed_order_status_error_redacts_sensitive_values() {
    let error = status_error_or_panic(&serde_json::json!({
        "error": "upstream echoed Authorization: Basic basic-secret accessToken=\"access-secret\" trace=0x0123456789abcdef0123456789abcdef01234567"
    }));

    assert!(error.contains("orderStatus error:"));
    assert!(error.contains("<redacted>"));
    assert!(error.contains("<redacted-hex>"));
    for secret in [
        "basic-secret",
        "access-secret",
        "0123456789abcdef0123456789abcdef01234567",
    ] {
        assert!(!error.contains(secret), "orderStatus error leaked {secret}");
    }
}

#[test]
fn canceled_status_is_not_definitive_no_fill() {
    let parsed = status_or_panic(&serde_json::json!({
        "status": "order",
        "order": {
            "status": "canceled",
            "order": {
                "oid": 42_u64
            }
        }
    }));

    assert!(parsed.is_no_fill_terminal());
    assert!(!parsed.is_definitive_no_fill_terminal());
}

#[test]
fn rejected_status_is_definitive_no_fill() {
    let parsed = status_or_panic(&serde_json::json!({
        "status": "order",
        "order": {
            "status": "rejected",
            "order": {
                "oid": 42_u64
            }
        }
    }));

    assert!(parsed.is_definitive_no_fill_terminal());
}
