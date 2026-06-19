use super::*;

#[test]
fn ws_book_parser_filters_nonfinite_nonpositive_levels() {
    let data = serde_json::json!({
        "levels": [
            [
                { "px": "100", "sz": "1" },
                { "px": "NaN", "sz": "1" },
                { "px": "0", "sz": "1" },
                { "px": "101", "sz": "-1" }
            ],
            [
                { "px": "101", "sz": "2" },
                { "px": "inf", "sz": "1" },
                { "px": "102", "sz": "0" }
            ]
        ]
    });

    let book = parse_ws_book(&data).expect("valid two-sided payload shape should parse");

    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.bids[0].px, 100.0);
    assert_eq!(book.bids[0].sz, 1.0);
    assert_eq!(book.asks.len(), 1);
    assert_eq!(book.asks[0].px, 101.0);
    assert_eq!(book.asks[0].sz, 2.0);
}

#[test]
fn book_level_debug_redacts_price_and_size() {
    let level = BookLevel {
        px: 12345.67,
        sz: 89.01,
    };

    let rendered = format!("{level:?}");

    assert!(rendered.contains("px: \"<redacted>\""));
    assert!(rendered.contains("sz: \"<redacted>\""));
    assert!(!rendered.contains("12345.67"));
    assert!(!rendered.contains("89.01"));
}

#[test]
fn order_book_debug_redacts_level_payloads() {
    let book = OrderBook {
        bids: vec![BookLevel {
            px: 12345.67,
            sz: 89.01,
        }],
        asks: vec![BookLevel {
            px: 12346.78,
            sz: 90.12,
        }],
    };

    let rendered = format!("{book:?}");

    assert!(rendered.contains("bids_len: 1"));
    assert!(rendered.contains("asks_len: 1"));
    assert!(rendered.contains("has_best_bid: true"));
    assert!(rendered.contains("has_best_ask: true"));
    for secret in ["12345.67", "89.01", "12346.78", "90.12"] {
        assert!(
            !rendered.contains(secret),
            "order book Debug leaked {secret}"
        );
    }
}

#[test]
fn ws_book_parser_rejects_missing_two_sided_levels() {
    assert!(parse_ws_book(&serde_json::json!({ "levels": [] })).is_none());
    assert!(parse_ws_book(&serde_json::json!({ "levels": [[]] })).is_none());
    assert!(parse_ws_book(&serde_json::json!({ "notLevels": [] })).is_none());
}

#[test]
fn rest_book_parser_reports_null_response_context() {
    let error = parse_order_book_response(&serde_json::Value::Null).unwrap_err();

    assert!(error.contains("l2Book returned null"));
    assert!(error.contains("unsupported"));
}

#[test]
fn rest_book_parser_reports_error_response_context() {
    let error = parse_order_book_response(&serde_json::json!({
        "error": "Unknown coin"
    }))
    .unwrap_err();

    assert_eq!(error, "l2Book error: Unknown coin");
}

#[test]
fn rest_book_parser_reports_unexpected_shape_context() {
    let error = parse_order_book_response(&serde_json::json!({
        "unexpected": true
    }))
    .unwrap_err();

    assert!(error.contains("Expected levels array"));
    assert!(error.contains("unexpected"));
}
