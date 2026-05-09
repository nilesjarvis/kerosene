use serde_json::json;

use super::tracked_trade_subscription;

#[test]
fn tracked_trade_subscription_rejects_empty_addresses() {
    assert!(tracked_trade_subscription(Vec::new()).is_none());
}

#[test]
fn tracked_trade_subscription_matches_hydromancer_wire_shape() {
    let (topic, payload) =
        tracked_trade_subscription(vec!["0xabc".to_string(), "0xdef".to_string()])
            .expect("subscription");

    assert_eq!(topic, "userFills:0xabc,0xdef");
    assert_eq!(
        payload,
        json!({
            "type": "subscribe",
            "subscription": {
                "type": "userFills",
                "addresses": ["0xabc", "0xdef"],
                "aggregateByTime": true
            }
        })
    );
}
