use reqwest::StatusCode;

use super::parse_candle_response;

#[test]
fn candle_response_null_is_empty() {
    let candles = parse_candle_response(StatusCode::OK, Some("application/json"), "null")
        .expect("null response");
    assert!(candles.is_empty());
}

#[test]
fn candle_response_rejects_non_json_content() {
    let err = parse_candle_response(StatusCode::OK, Some("text/html"), "<html></html>")
        .expect_err("non-json response");
    assert!(err.contains("instead of JSON"));
}

#[test]
fn candle_response_rejects_unexpected_json_object() {
    let err = parse_candle_response(StatusCode::OK, Some("application/json"), "{\"ok\":true}")
        .expect_err("object response");
    assert!(err.contains("Unexpected candle response"));
}

#[test]
fn candle_response_parses_and_normalizes_candles() {
    let text = r#"[
        {"t":2000,"T":2059,"o":"2","h":"3","l":"1","c":"2.5","v":"10"},
        {"t":1000,"T":1059,"o":1,"h":2,"l":0.5,"c":1.5,"v":8}
    ]"#;
    let candles =
        parse_candle_response(StatusCode::OK, Some("application/json"), text).expect("candles");

    assert_eq!(
        candles
            .iter()
            .map(|candle| (candle.open_time, candle.close))
            .collect::<Vec<_>>(),
        vec![(1000, 1.5), (2000, 2.5)]
    );
}
