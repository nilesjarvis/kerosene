use super::ExchangeResponse;
use serde_json::{Value, json};

#[test]
fn deserialization_preserves_typed_fields_and_exact_fallback_bodies() {
    let statuses = vec![
        Value::Null,
        json!(7),
        json!("success"),
        json!({"filled": {"oid": 42, "totalSz": "1.25"}, "extra": [true, {"nested": "value"}]}),
    ];
    for status in ["ok", "err"] {
        for (body, expected_type, expected_statuses) in [
            (
                json!({"type": "order", "data": {"statuses": statuses.clone(), "extra": "ignored"}, "extra": [1, 2]}),
                "order",
                Some(statuses.clone()),
            ),
            (
                json!({"type": "cancel", "data": {"statuses": []}}),
                "cancel",
                Some(Vec::new()),
            ),
            (json!({"type": "default"}), "default", None),
            (json!({"type": "order", "data": null}), "order", None),
            (
                json!(["order", {"statuses": statuses.clone()}]),
                "order",
                Some(statuses.clone()),
            ),
            (
                json!({"type": "order", "data": [statuses.clone()]}),
                "order",
                Some(statuses.clone()),
            ),
        ] {
            let response: ExchangeResponse =
                serde_json::from_value(json!({"status": status, "response": body}))
                    .expect("typed response");
            assert_eq!(response.status, status);
            assert!(response.raw_response.is_none());
            let inner = response.response.expect("typed body");
            assert_eq!(inner.response_type, expected_type);
            assert_eq!(inner.data.map(|data| data.statuses), expected_statuses);
        }

        for body in [
            json!("schema-shifted"),
            json!(false),
            json!(123),
            json!([]),
            json!({}),
            json!({"type": 1, "data": {"statuses": []}}),
            json!({"type": "order", "data": {}}),
            json!({"type": "order", "data": {"statuses": null}}),
            json!({"type": "order", "data": {"statuses": "schema-shifted"}, "extra": [1, {"nested": true}]}),
        ] {
            let response: ExchangeResponse =
                serde_json::from_value(json!({"status": status, "response": body.clone()}))
                    .expect("raw fallback");
            assert_eq!(response.status, status);
            assert!(response.response.is_none());
            assert_eq!(response.raw_response, Some(body));
        }
    }

    for envelope in [
        json!({"status": "ok"}),
        json!({"status": "ok", "response": null}),
    ] {
        let response: ExchangeResponse = serde_json::from_value(envelope).expect("absent body");
        assert!(response.response.is_none());
        assert!(response.raw_response.is_none());
        assert_eq!(response.summary(), "No response body");
        assert!(response.is_ambiguous_order_result());
        assert!(!response.is_fully_filled());
        assert!(!response.reports_filled());
        assert_eq!(response.filled_total_size(), None);
    }
}
