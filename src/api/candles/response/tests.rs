use reqwest::StatusCode;

use super::parse_candle_response;

#[test]
fn candle_response_null_is_empty() {
    let candles = parse_candle_response(StatusCode::OK, Some("application/json"), "null", false)
        .expect("null response");
    assert!(candles.is_empty());
}

#[test]
fn candle_response_rejects_non_json_content() {
    let err = parse_candle_response(StatusCode::OK, Some("text/html"), "<html></html>", false)
        .expect_err("non-json response");
    assert!(err.contains("instead of JSON"));
}

#[test]
fn candle_response_rejects_unexpected_json_object() {
    let err = parse_candle_response(
        StatusCode::OK,
        Some("application/json"),
        "{\"ok\":true}",
        false,
    )
    .expect_err("object response");
    assert!(err.contains("Unexpected candle response"));
}

#[test]
fn authenticated_candle_response_errors_redact_echoed_secrets() {
    let text = concat!(
        "Authorization: Bearer hydro-secret ",
        r#"{"token":"query-secret","api_key":"json-secret"}"#
    );
    let err = parse_candle_response(
        StatusCode::UNAUTHORIZED,
        Some("application/json"),
        text,
        true,
    )
    .expect_err("auth error");

    assert!(err.contains("Candle request failed"));
    assert!(err.contains("<redacted>"));
    for secret in ["hydro-secret", "query-secret", "json-secret"] {
        assert!(!err.contains(secret), "leaked {secret}");
    }
}

#[test]
fn unauthenticated_candle_response_errors_keep_plain_snippet() {
    let err = parse_candle_response(
        StatusCode::BAD_GATEWAY,
        Some("text/plain"),
        "upstream plain body",
        false,
    )
    .expect_err("plain error");

    assert!(err.contains("upstream plain body"));
}

#[test]
fn candle_response_parses_and_normalizes_candles() {
    let text = r#"[
        {"t":2000,"T":2059,"o":"2","h":"3","l":"1","c":"2.5","v":"10"},
        {"t":1000,"T":1059,"o":1,"h":2,"l":0.5,"c":1.5,"v":8}
    ]"#;
    let candles = parse_candle_response(StatusCode::OK, Some("application/json"), text, false)
        .expect("candles");

    assert_eq!(
        candles
            .iter()
            .map(|candle| (candle.open_time, candle.close))
            .collect::<Vec<_>>(),
        vec![(1000, 1.5), (2000, 2.5)]
    );
}
