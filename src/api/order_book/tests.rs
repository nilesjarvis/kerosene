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
fn book_parsers_preserve_numeric_forms_order_and_empty_sides() {
    let side = serde_json::json!([
        { "px": 100, "sz": "2.5", "ignored": { "nested": [null, true] } },
        ["102.5", 3],
        { "px": "1.01e2", "sz": 1.25 },
        { "px": -1, "sz": 1 },
        { "px": 100, "sz": "NaN" },
        { "px": 100, "sz": "inf" },
        { "px": 100, "sz": -0.0 },
        { "px": "1e309", "sz": 1 }
    ]);
    for side_index in 0..2 {
        let mut data = serde_json::json!({ "levels": [[], [], "ignored extra side"] });
        data["levels"][side_index] = side.clone();
        for book in [
            parse_ws_book(&data).expect("valid websocket book"),
            parse_order_book_response(&data).expect("valid REST book"),
        ] {
            let sides = [book.bids, book.asks];
            let actual: Vec<_> = sides[side_index]
                .iter()
                .map(|level| (level.px, level.sz))
                .collect();
            assert_eq!(actual, [(100.0, 2.5), (102.5, 3.0), (101.0, 1.25)]);
            assert!(sides[1 - side_index].is_empty());
        }
    }
}

#[test]
fn book_parsers_reject_malformed_sides_without_returning_partial_levels() {
    for (side, reason) in [
        (
            serde_json::json!(null),
            "invalid type: null, expected a sequence",
        ),
        (serde_json::json!([{}]), "missing field `px`"),
        (serde_json::json!([{ "px": 100 }]), "missing field `sz`"),
        (
            serde_json::json!([[100, 1, 2]]),
            "invalid length 3, expected fewer elements in array",
        ),
        (
            serde_json::json!([{ "px": 100, "sz": 1 }, { "px": "bad", "sz": 2 }]),
            "invalid float literal",
        ),
        (
            serde_json::json!([{ "px": 0, "sz": true }]),
            "invalid type: boolean `true`, expected a string or number representing an f64",
        ),
    ] {
        for (side_index, side_name) in [(0, "bids"), (1, "asks")] {
            let mut data = serde_json::json!({ "levels": [[], []] });
            data["levels"][side_index] = side.clone();
            assert!(parse_ws_book(&data).is_none());
            assert_eq!(
                parse_order_book_response(&data).expect_err("malformed side"),
                format!("Failed to parse {side_name}: {reason}")
            );
        }
    }
}

#[test]
fn book_parsers_keep_rest_error_precedence_separate_from_websocket_shape() {
    let mut data = serde_json::json!({ "error": "Unknown coin", "levels": [[], []] });
    assert_eq!(
        parse_order_book_response(&data).expect_err("REST error takes precedence"),
        "l2Book error: Unknown coin"
    );
    assert!(parse_ws_book(&data).is_some());

    data["error"] = serde_json::json!(42);
    assert!(parse_order_book_response(&data).is_ok());

    data["levels"] = serde_json::json!([null, null]);
    assert_eq!(
        parse_order_book_response(&data).expect_err("bid error takes precedence"),
        "Failed to parse bids: invalid type: null, expected a sequence"
    );
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
