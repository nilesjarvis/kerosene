use super::parse_funding_history_response;

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
