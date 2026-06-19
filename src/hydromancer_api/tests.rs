use super::{FundingRatePoint, parse_funding_history_response};

#[test]
fn parses_and_sorts_funding_history() {
    let points = parse_funding_history_response(
        r#"[
            {"coin":"BTC","fundingRate":"0.0000125","time":1778202000069},
            {"coin":"BTC","fundingRate":"-0.000003","time":1778198400069}
        ]"#,
    )
    .expect("valid funding response");

    assert_eq!(points.len(), 2);
    assert_eq!(points[0].time_ms, 1778198400069);
    assert_eq!(points[0].rate, -0.000003);
    assert_eq!(points[1].rate, 0.0000125);
}

#[test]
fn funding_rate_point_debug_redacts_rate() {
    let point = FundingRatePoint {
        time_ms: 1778198400069,
        rate: 0.0000125,
    };

    let rendered = format!("{point:?}");

    assert!(rendered.contains("time_ms: 1778198400069"));
    assert!(rendered.contains("rate: \"<redacted>\""));
    assert!(!rendered.contains("0.0000125"));
}

#[test]
fn rejects_oversized_funding_rate() {
    let err = parse_funding_history_response(
        r#"[{"coin":"BTC","fundingRate":"1.7e308","time":1778198400069}]"#,
    )
    .expect_err("oversized rate should fail");

    assert!(err.contains("unrealistic rate magnitude"));
}

#[test]
fn rejects_malformed_funding_rate() {
    let err = parse_funding_history_response(
        r#"[{"coin":"BTC","fundingRate":"not-a-number","time":1778198400069}]"#,
    )
    .expect_err("invalid rate should fail");

    assert!(err.contains("Invalid Hydromancer funding rate"));
}

#[test]
fn malformed_funding_response_error_includes_redacted_snippet() {
    let err = parse_funding_history_response(
        r#"upstream failure Authorization: Bearer hydro-secret api_key="json-secret" txid=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"#,
    )
    .expect_err("malformed response should fail");

    assert!(err.contains("Hydromancer funding response parse failed"));
    assert!(err.contains("Response:"));
    assert!(err.contains("<redacted>"));
    assert!(err.contains("<redacted-hex>"));
    for secret in [
        "hydro-secret",
        "json-secret",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        assert!(!err.contains(secret), "funding parse error leaked {secret}");
    }
}
