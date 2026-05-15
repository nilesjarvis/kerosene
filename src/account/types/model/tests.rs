use super::{AccountAbstractionMode, OpenOrder, SpotBalance, UserFill};

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

#[test]
fn user_fill_preserves_optional_order_id_metadata() {
    let fill: UserFill = serde_json::from_value(serde_json::json!({
        "coin": "BTC",
        "px": "100",
        "sz": "0.1",
        "side": "B",
        "time": 1_u64,
        "oid": 42_u64,
        "dir": "Open Long",
        "closedPnl": "0",
        "fee": "0.01"
    }))
    .expect("fill should deserialize");

    assert_eq!(fill.oid, Some(42));
}

#[test]
fn user_fill_accepts_missing_order_id_metadata() {
    let fill: UserFill = serde_json::from_value(serde_json::json!({
        "coin": "BTC",
        "px": "100",
        "sz": "0.1",
        "side": "B",
        "time": 1_u64,
        "dir": "Open Long",
        "closedPnl": "0",
        "fee": "0.01"
    }))
    .expect("fill should deserialize");

    assert_eq!(fill.oid, None);
}

#[test]
fn account_abstraction_mode_parses_known_api_values() {
    assert_eq!(
        AccountAbstractionMode::from_api_value("portfolioMargin"),
        AccountAbstractionMode::PortfolioMargin
    );
    assert_eq!(
        AccountAbstractionMode::from_api_value("unifiedAccount"),
        AccountAbstractionMode::UnifiedAccount
    );
    assert_eq!(
        AccountAbstractionMode::from_api_value("dexAbstraction"),
        AccountAbstractionMode::DexAbstraction
    );
}

#[test]
fn spot_balance_preserves_optional_token_index() {
    let balance: SpotBalance = serde_json::from_value(serde_json::json!({
        "coin": "USDC",
        "token": 0,
        "total": "10",
        "hold": "2",
        "entryNtl": "0"
    }))
    .expect("spot balance should deserialize");

    assert_eq!(balance.token, Some(0));
}
