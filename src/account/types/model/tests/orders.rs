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
