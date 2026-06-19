use super::*;

#[test]
fn open_order_preserves_reduce_only_metadata_when_present() {
    let order = open_order_or_panic(open_order_value(serde_json::json!({
        "reduceOnly": true
    })));

    assert_eq!(order.reduce_only, Some(true));
}

#[test]
fn open_order_marks_reduce_only_metadata_unknown_when_absent() {
    let order = open_order_or_panic(open_order_value(serde_json::json!({})));

    assert_eq!(order.reduce_only, None);
}

#[test]
fn open_order_preserves_trigger_metadata_when_present() {
    let order = open_order_or_panic(open_order_value(serde_json::json!({
        "isTrigger": true,
        "orderType": "Take Profit Market",
        "tif": "Gtc",
        "triggerPx": "123.45"
    })));

    assert_eq!(order.is_trigger, Some(true));
    assert_eq!(order.order_type.as_deref(), Some("Take Profit Market"));
    assert_eq!(order.tif.as_deref(), Some("Gtc"));
    assert_eq!(order.trigger_px.as_deref(), Some("123.45"));
}

#[test]
fn open_order_debug_redacts_order_payload() {
    let order = OpenOrder {
        coin: "SECRETORDERCOIN".to_string(),
        side: "B".to_string(),
        limit_px: "limit-price-secret".to_string(),
        sz: "size-secret".to_string(),
        oid: 424242,
        timestamp: 123,
        reduce_only: Some(true),
        is_trigger: Some(true),
        order_type: Some("Take Profit Market".to_string()),
        tif: Some("Gtc".to_string()),
        trigger_px: Some("trigger-price-secret".to_string()),
    };

    let rendered = format!("{order:?}");

    assert!(rendered.contains("OpenOrder"));
    assert!(rendered.contains("side: \"B\""));
    assert!(rendered.contains("timestamp: 123"));
    assert!(rendered.contains("reduce_only: Some(true)"));
    assert!(rendered.contains("order_type: Some(\"Take Profit Market\")"));
    for secret in [
        "SECRETORDERCOIN",
        "limit-price-secret",
        "size-secret",
        "424242",
        "trigger-price-secret",
    ] {
        assert!(
            !rendered.contains(secret),
            "open order Debug leaked {secret}"
        );
    }
}
