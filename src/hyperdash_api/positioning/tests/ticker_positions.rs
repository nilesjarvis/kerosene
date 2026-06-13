use super::*;

#[test]
fn ticker_positions_parse_nullable_identity_and_liquidation_fields() {
    let parsed = ticker_positions_or_panic(
        r#"{
          "data": {
            "analytics": {
              "perpsTickerPositions": {
                "coin": "HYPE",
                "positions": [{
                  "address": "0xabc0000000000000000000000000000000000000",
                  "displayName": null,
                  "label": "Whale",
                  "tag": "swing",
                  "verified": null,
                  "copyScore": 61.5,
                  "size": 12.5,
                  "notionalSize": 500.25,
                  "entryPrice": 30.0,
                  "liquidationPrice": null,
                  "unrealizedPnl": 125.75,
                  "fundingPnl": -4.5,
                  "accountValue": 1000.0
                }],
                "totalLongNotional": 600.0,
                "totalShortNotional": 400.0,
                "totalNotional": 1000.0,
                "longCount": 3,
                "shortCount": 2,
                "totalCount": 5,
                "hasMore": true,
                "timestamp": "2026-05-18T11:52:39.585Z"
              }
            }
          }
        }"#,
    );

    assert_eq!(parsed.coin, "HYPE");
    assert_eq!(parsed.total_count, 5);
    assert!(parsed.has_more);
    assert_eq!(parsed.positions.len(), 1);
    assert_eq!(parsed.positions[0].label.as_deref(), Some("Whale"));
    assert_eq!(parsed.positions[0].liquidation_price, None);
}

#[test]
fn ticker_positions_reports_graphql_errors_without_data() {
    let error = ticker_positions_error_or_panic(
        r#"{"errors":[{"message":"invalid api key"}],"data":null}"#,
    );

    assert!(error.contains("authentication failed"));
}

#[test]
fn ticker_positions_reports_graphql_errors_for_missing_partial_field() {
    let error = ticker_positions_error_or_panic(
        r#"{
          "data": {"analytics": {"perpsTickerPositions": null}},
          "errors": [{"message": "coin not found"}]
        }"#,
    );

    assert!(error.contains("coin not found"));
}

#[test]
fn ticker_positions_redacts_sensitive_graphql_errors() {
    let error = ticker_positions_error_or_panic(
        r#"{
          "errors": [{
            "message": "provider echoed api_key=\"hyper-secret\" Bearer bearer-secret trace=0x0123456789abcdef0123456789abcdef01234567"
          }],
          "data": null
        }"#,
    );

    assert!(error.contains("<redacted>"));
    assert!(error.contains("<redacted-hex>"));
    for secret in [
        "hyper-secret",
        "bearer-secret",
        "0123456789abcdef0123456789abcdef01234567",
    ] {
        assert!(!error.contains(secret), "positioning error leaked {secret}");
    }
}
