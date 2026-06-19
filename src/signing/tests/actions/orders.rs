use super::json_value;
use super::{
    CLIENT_ORDER_ID, ExchangeOrderKind, build_order_action, build_order_action_with_cloid,
    build_update_leverage_action,
};

#[test]
fn build_order_action_serializes_limit_payload_for_exchange() {
    let action = build_order_action(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        ExchangeOrderKind::Limit,
        false,
    );
    let json = json_value(action, "order action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "order",
            "orders": [{
                "a": 7,
                "b": true,
                "p": "123.45",
                "s": "0.25",
                "r": false,
                "t": {
                    "limit": {
                        "tif": "Gtc"
                    }
                }
            }],
            "grouping": "na"
        })
    );
}

#[test]
fn build_order_action_can_include_client_order_id() {
    let action = build_order_action_with_cloid(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        ExchangeOrderKind::LimitIoc,
        false,
        Some(CLIENT_ORDER_ID.to_string()),
    );
    let json = json_value(action, "order action should serialize");

    assert_eq!(json["orders"][0]["c"], CLIENT_ORDER_ID);
    assert_eq!(json["orders"][0]["t"]["limit"]["tif"], "Ioc");
}

#[test]
fn order_action_debug_redacts_order_values_and_cloid() {
    let action = build_order_action_with_cloid(
        7,
        true,
        "price-secret".to_string(),
        "size-secret".to_string(),
        ExchangeOrderKind::LimitIoc,
        false,
        Some(CLIENT_ORDER_ID.to_string()),
    );

    let rendered = format!("{action:?}");

    assert!(rendered.contains("OrderAction"));
    assert!(rendered.contains("orders_count: 1"));
    assert!(!rendered.contains("price-secret"));
    assert!(!rendered.contains("size-secret"));
    assert!(!rendered.contains(CLIENT_ORDER_ID));
}

#[test]
fn build_order_action_uses_ioc_for_market_and_limit_ioc_and_gtc_for_limit() {
    let market = build_order_action(
        1,
        false,
        "100".to_string(),
        "2".to_string(),
        ExchangeOrderKind::Market,
        true,
    );
    let limit_ioc = build_order_action(
        1,
        true,
        "101".to_string(),
        "2".to_string(),
        ExchangeOrderKind::LimitIoc,
        false,
    );
    let limit = build_order_action(
        1,
        true,
        "99".to_string(),
        "2".to_string(),
        ExchangeOrderKind::Limit,
        false,
    );

    let market_json = json_value(market, "market action should serialize");
    let limit_ioc_json = json_value(limit_ioc, "limit IOC action should serialize");
    let limit_json = json_value(limit, "limit action should serialize");

    assert_eq!(market_json["orders"][0]["t"]["limit"]["tif"], "Ioc");
    assert_eq!(market_json["orders"][0]["r"], true);
    assert_eq!(limit_ioc_json["orders"][0]["t"]["limit"]["tif"], "Ioc");
    assert_eq!(limit_json["orders"][0]["t"]["limit"]["tif"], "Gtc");
}

#[test]
fn build_update_leverage_action_serializes_payload_for_exchange() {
    let action = build_update_leverage_action(110_003, false, 7);
    let json = json_value(action, "update leverage action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "updateLeverage",
            "asset": 110_003,
            "isCross": false,
            "leverage": 7
        })
    );
}
