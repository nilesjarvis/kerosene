use super::json_value;
use super::{CLIENT_ORDER_ID, OrderKind, build_order_action, build_order_action_with_cloid};

#[test]
fn build_order_action_serializes_limit_payload_for_exchange() {
    let action = build_order_action(
        7,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        OrderKind::Limit,
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
        OrderKind::LimitIoc,
        false,
        Some(CLIENT_ORDER_ID.to_string()),
    );
    let json = json_value(action, "order action should serialize");

    assert_eq!(json["orders"][0]["c"], CLIENT_ORDER_ID);
    assert_eq!(json["orders"][0]["t"]["limit"]["tif"], "Ioc");
}

#[test]
fn build_order_action_uses_ioc_for_market_and_limit_ioc_and_gtc_for_chase() {
    let market = build_order_action(
        1,
        false,
        "100".to_string(),
        "2".to_string(),
        OrderKind::Market,
        true,
    );
    let limit_ioc = build_order_action(
        1,
        true,
        "101".to_string(),
        "2".to_string(),
        OrderKind::LimitIoc,
        false,
    );
    let chase = build_order_action(
        1,
        true,
        "99".to_string(),
        "2".to_string(),
        OrderKind::Chase,
        false,
    );

    let market_json = json_value(market, "market action should serialize");
    let limit_ioc_json = json_value(limit_ioc, "limit IOC action should serialize");
    let chase_json = json_value(chase, "chase action should serialize");

    assert_eq!(market_json["orders"][0]["t"]["limit"]["tif"], "Ioc");
    assert_eq!(market_json["orders"][0]["r"], true);
    assert_eq!(limit_ioc_json["orders"][0]["t"]["limit"]["tif"], "Ioc");
    assert_eq!(chase_json["orders"][0]["t"]["limit"]["tif"], "Gtc");
}
