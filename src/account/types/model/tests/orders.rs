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
