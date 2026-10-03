use super::*;
use serde_json::json;

#[test]
fn capacity_errors_support_legacy_messages_and_structured_codes() {
    for error in [
        "Too many subscriptions (10)",
        "Maximum connections reached for API key",
        "subscription_limit_exceeded",
        "connection_limit",
        "candle subscription limit reached",
        "Orderbook limit exceeded",
        "subscription denied: quota exceeded",
        "quota exhausted",
        "HTTP error: 429 Too Many Requests",
        "rate_limit_exceeded",
    ] {
        assert!(is_capacity_error(error), "{error}");
        assert!(is_capacity_message("error", &json!({"message":error})));
        assert!(is_capacity_message(
            "subscriptionUpdate",
            &json!({"failed":[{"code":error}]})
        ));
    }
    assert!(is_capacity_message("error", &json!({"error":{"code":429}})));
}

#[test]
fn unrelated_errors_successful_feedback_and_data_do_not_shed_streams() {
    for error in [
        "HTTP 401 Unauthorized",
        "heartbeat timeout after 95s",
        "invalid interval",
        "stream closed",
    ] {
        assert!(!is_capacity_error(error));
    }
    assert!(!is_capacity_message(
        "subscriptionUpdate",
        &json!({
            "subscribed":["candle"], "failed":[], "warnings":[{"message":"quota exceeded"}]
        })
    ));
    assert!(!is_capacity_message(
        "error",
        &json!({"operation":"unsubscribe", "message":"subscription limit"})
    ));
    assert!(!is_capacity_message(
        "candle",
        &json!({"message":"quota exceeded"})
    ));
}

#[test]
fn fallback_preserves_hydromancer_only_and_unknown_streams() {
    for kind in ["liquidationFills", "userFills", "futureFeed"] {
        assert!(!has_market_fallback(&json!({"subscription":{"type":kind}})));
    }
    assert!(!has_market_fallback(
        &json!({"subscription":{"type":"candle", "interval":"1s"}})
    ));
    assert!(has_market_fallback(
        &json!({"subscription":{"type":"candle", "interval":"1m"}})
    ));
    for kind in ["l2Book", "activeAssetCtx"] {
        assert!(has_market_fallback(&json!({"subscription":{"type":kind}})));
    }
}
