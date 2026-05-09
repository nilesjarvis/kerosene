use super::OpenOrder;

fn open_order_value(extra: serde_json::Value) -> serde_json::Value {
    let mut order = serde_json::json!({
        "coin": "BTC",
        "side": "B",
        "limitPx": "100",
        "sz": "0.1",
        "oid": 42_u64,
        "timestamp": 1_u64
    });
    let order_obj = order
        .as_object_mut()
        .expect("test order should serialize as object");
    if let Some(extra_obj) = extra.as_object() {
        for (key, value) in extra_obj {
            order_obj.insert(key.clone(), value.clone());
        }
    }
    order
}

#[test]
fn open_order_preserves_reduce_only_metadata_when_present() {
    let order: OpenOrder = serde_json::from_value(open_order_value(serde_json::json!({
        "reduceOnly": true
    })))
    .expect("open order should deserialize");

    assert_eq!(order.reduce_only, Some(true));
}

#[test]
fn open_order_marks_reduce_only_metadata_unknown_when_absent() {
    let order: OpenOrder = serde_json::from_value(open_order_value(serde_json::json!({})))
        .expect("open order should deserialize");

    assert_eq!(order.reduce_only, None);
}
