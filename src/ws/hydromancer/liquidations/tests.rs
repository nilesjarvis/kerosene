use serde_json::json;

use super::liquidation_subscription;

#[test]
fn liquidation_subscription_matches_hydromancer_wire_shape() {
    let (topic, payload) = liquidation_subscription();

    assert_eq!(topic, "liquidationFills");
    assert_eq!(
        payload,
        json!({
            "type": "subscribe",
            "subscription": { "type": "liquidationFills" }
        })
    );
}
