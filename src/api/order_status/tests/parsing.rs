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
fn parses_missing_order_status() {
    let parsed = status_or_panic(&serde_json::json!({
        "status": "unknownOid"
    }));

    assert!(parsed.is_missing());
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
