use super::{parse_open_interest_caps, parse_perps_at_open_interest_cap};
use serde_json::json;
use std::collections::HashSet;

#[test]
fn cap_status_normalizes_native_and_builder_symbols() {
    for (dex, response, expected) in [
        ("", json!(["BTC", "ETH", "BTC"]), vec!["BTC", "ETH"]),
        (
            "xyz",
            json!(["AAPL", "xyz:NVDA", "xyz:AAPL"]),
            vec!["xyz:AAPL", "xyz:NVDA"],
        ),
        ("xyz", json!([]), vec![]),
    ] {
        assert_eq!(
            parse_perps_at_open_interest_cap(&response, dex).expect("valid cap status"),
            expected
                .into_iter()
                .map(str::to_string)
                .collect::<HashSet<_>>()
        );
    }
}

#[test]
fn invalid_cap_status_is_not_treated_as_confirmation_markets_are_below_cap() {
    for response in [
        json!(null),
        json!({}),
        json!([null]),
        json!(["BTC", 42]),
        json!([""]),
    ] {
        assert!(parse_perps_at_open_interest_cap(&response, "").is_err());
    }
    for (dex, response) in [
        ("", json!(["xyz:AAPL"])),
        ("xyz", json!(["abc:AAPL"])),
        ("xyz", json!(["xyz:"])),
        ("xyz", json!(["xyz:abc:AAPL"])),
    ] {
        assert!(parse_perps_at_open_interest_cap(&response, dex).is_err());
    }
}

#[test]
fn caps_use_per_asset_limits_and_normalize_builder_symbols() {
    let caps = parse_open_interest_caps(
        &json!({
            "totalOiCap": "10000000000.0",
            "oiSzCapPerPerp": "20000000000.0",
            "coinToOiCap": [["xyz:AAPL", "200000000.0"], ["NVDA", 25_000_000.0], ["ZERO", "0"]]
        }),
        "xyz",
    )
    .expect("valid caps");
    assert_eq!(caps.len(), 3);
    assert_eq!(caps["xyz:AAPL"], 200_000_000.0);
    assert_eq!(caps["xyz:NVDA"], 25_000_000.0);
    assert_eq!(caps["xyz:ZERO"], 0.0);
}

#[test]
fn invalid_caps_and_other_dexes_do_not_become_market_limits() {
    let caps = parse_open_interest_caps(
        &json!({"coinToOiCap": [
            ["xyz:GOOD", "1000"], ["other:GOOD", "2000"], ["", "3"], ["xyz:", "4"],
            ["NEGATIVE", "-1"], ["NAN", "NaN"], ["INF", "inf"], ["OVERFLOW", "1e400"],
            ["MISSING", null], ["BAD", "bad"], ["SHORT"], ["LONG", "1", "2"], null
        ]}),
        "xyz",
    )
    .expect("valid envelope");
    assert_eq!(
        caps.into_iter().collect::<Vec<_>>(),
        [("xyz:GOOD".to_string(), 1000.0)]
    );
    assert!(parse_open_interest_caps(&json!({"totalOiCap": "5000"}), "xyz").is_err());
    assert!(
        parse_open_interest_caps(&json!({"coinToOiCap": []}), "xyz")
            .expect("empty limits")
            .is_empty()
    );
}

#[tokio::test]
async fn native_dex_does_not_issue_an_unsupported_limits_request() {
    assert!(
        super::fetch_open_interest_caps(String::new())
            .await
            .is_err()
    );
}
