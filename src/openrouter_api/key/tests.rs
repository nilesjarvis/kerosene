use super::*;

#[test]
fn key_status_response_parses_full_payload() {
    let text = r#"{
        "data": {
            "label": "kerosene",
            "usage": 25.5,
            "limit": 100.0,
            "limit_remaining": 74.5,
            "is_free_tier": false,
            "rate_limit": {"requests": 10, "interval": "10s"}
        }
    }"#;

    let status = parse_key_status_response(text).expect("full key status should parse");

    assert_eq!(
        status,
        OpenRouterKeyStatus {
            usage_usd: 25.5,
            limit_usd: Some(100.0),
            limit_remaining_usd: Some(74.5),
            is_free_tier: false,
        }
    );
}

#[test]
fn key_status_response_defaults_missing_fields() {
    let status =
        parse_key_status_response(r#"{"data": {}}"#).expect("minimal key status should parse");

    assert_eq!(
        status,
        OpenRouterKeyStatus {
            usage_usd: 0.0,
            limit_usd: None,
            limit_remaining_usd: None,
            is_free_tier: false,
        }
    );
}

#[test]
fn key_status_response_without_data_is_an_error() {
    let error =
        parse_key_status_response(r#"{"unexpected": true}"#).expect_err("missing data should fail");

    assert!(error.contains("key check parse failed"));
}

#[test]
fn key_status_fetch_rejects_missing_key_before_any_io() {
    let error = futures::executor::block_on(fetch_key_status(Zeroizing::new(String::new())))
        .expect_err("missing key should fail");

    assert!(error.contains("OpenRouter API key is required"));
}
