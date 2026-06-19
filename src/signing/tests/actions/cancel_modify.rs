use super::{
    CLIENT_ORDER_ID, build_cancel_action, build_cancel_by_cloid_action, build_modify_action,
    json_value,
};

#[test]
fn build_cancel_action_serializes_exchange_payload() {
    let action = build_cancel_action(3, 9001);
    let json = json_value(action, "cancel action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "cancel",
            "cancels": [{
                "a": 3,
                "o": 9001
            }]
        })
    );
}

#[test]
fn build_cancel_by_cloid_action_serializes_exchange_payload() {
    let action = build_cancel_by_cloid_action(3, CLIENT_ORDER_ID.to_string());
    let json = json_value(action, "cancel by cloid action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "cancelByCloid",
            "cancels": [{
                "asset": 3,
                "cloid": CLIENT_ORDER_ID
            }]
        })
    );
}

#[test]
fn build_modify_action_serializes_exchange_payload() {
    let action = build_modify_action(
        9001,
        3,
        true,
        "123.45".to_string(),
        "0.25".to_string(),
        false,
    );
    let json = json_value(action, "modify action should serialize");

    assert_eq!(
        json,
        serde_json::json!({
            "type": "batchModify",
            "modifies": [{
                "oid": 9001,
                "order": {
                    "a": 3,
                    "b": true,
                    "p": "123.45",
                    "s": "0.25",
                    "r": false,
                    "t": {
                        "limit": {
                            "tif": "Gtc"
                        }
                    }
                }
            }]
        })
    );
}

#[test]
fn cancel_and_modify_action_debug_redacts_exchange_identifiers_and_order_values() {
    let cancel = build_cancel_action(3, 9001);
    let cancel_by_cloid = build_cancel_by_cloid_action(3, CLIENT_ORDER_ID.to_string());
    let modify = build_modify_action(
        9001,
        3,
        true,
        "price-secret".to_string(),
        "size-secret".to_string(),
        false,
    );

    let cancel_debug = format!("{cancel:?}");
    let cancel_by_cloid_debug = format!("{cancel_by_cloid:?}");
    let modify_debug = format!("{modify:?}");

    assert!(cancel_debug.contains("CancelAction"));
    assert!(cancel_debug.contains("cancels_count: 1"));
    assert!(!cancel_debug.contains("9001"));

    assert!(cancel_by_cloid_debug.contains("CancelByCloidAction"));
    assert!(cancel_by_cloid_debug.contains("cancels_count: 1"));
    assert!(!cancel_by_cloid_debug.contains(CLIENT_ORDER_ID));

    assert!(modify_debug.contains("ModifyAction"));
    assert!(modify_debug.contains("modifies_count: 1"));
    assert!(!modify_debug.contains("9001"));
    assert!(!modify_debug.contains("price-secret"));
    assert!(!modify_debug.contains("size-secret"));
}
