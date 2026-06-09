use super::{AccountAbstractionMode, OpenOrder, SpotBalance, UserFill};

mod abstraction;
mod fills;
mod orders;
mod spot;

fn open_order_value(extra: serde_json::Value) -> serde_json::Value {
    let mut order = serde_json::json!({
        "coin": "BTC",
        "side": "B",
        "limitPx": "100",
        "sz": "0.1",
        "oid": 42_u64,
        "timestamp": 1_u64
    });
    let Some(order_obj) = order.as_object_mut() else {
        panic!("test order should serialize as object");
    };
    if let Some(extra_obj) = extra.as_object() {
        for (key, value) in extra_obj {
            order_obj.insert(key.clone(), value.clone());
        }
    }
    order
}

fn user_fill_value_with_oid(oid: Option<u64>) -> serde_json::Value {
    let mut fill = serde_json::json!({
        "coin": "BTC",
        "px": "100",
        "sz": "0.1",
        "side": "B",
        "time": 1_u64,
        "dir": "Open Long",
        "closedPnl": "0",
        "fee": "0.01"
    });
    if let Some(oid) = oid {
        let Some(fill_obj) = fill.as_object_mut() else {
            panic!("test fill should serialize as object");
        };
        fill_obj.insert("oid".to_string(), serde_json::json!(oid));
    }
    fill
}

fn user_fill_value_with_identity(tid: u64, hash: &str) -> serde_json::Value {
    let mut fill = user_fill_value_with_oid(Some(42));
    let Some(fill_obj) = fill.as_object_mut() else {
        panic!("test fill should serialize as object");
    };
    fill_obj.insert("tid".to_string(), serde_json::json!(tid));
    fill_obj.insert("hash".to_string(), serde_json::json!(hash));
    fill
}

fn open_order_or_panic(value: serde_json::Value) -> OpenOrder {
    match serde_json::from_value(value) {
        Ok(order) => order,
        Err(error) => panic!("open order should deserialize: {error}"),
    }
}

fn user_fill_or_panic(value: serde_json::Value) -> UserFill {
    match serde_json::from_value(value) {
        Ok(fill) => fill,
        Err(error) => panic!("fill should deserialize: {error}"),
    }
}

fn spot_balance_or_panic(value: serde_json::Value) -> SpotBalance {
    match serde_json::from_value(value) {
        Ok(balance) => balance,
        Err(error) => panic!("spot balance should deserialize: {error}"),
    }
}
