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
