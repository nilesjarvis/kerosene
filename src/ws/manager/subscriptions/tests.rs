use serde_json::json;

use super::{ActiveWsSubscriptions, WsUnsubscribeResult};

#[test]
fn subscribe_returns_payload_only_for_new_topic() {
    let mut subscriptions = ActiveWsSubscriptions::default();

    assert_eq!(
        subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"})),
        Some(json!({"method":"subscribe"}))
    );
    assert_eq!(
        subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"})),
        None
    );
    assert_eq!(
        subscriptions.payloads().cloned().collect::<Vec<_>>(),
        vec![json!({"method":"subscribe"})]
    );
}

#[test]
fn subscribe_same_topic_with_different_payload_tracks_independent_subscription() {
    let mut subscriptions = ActiveWsSubscriptions::default();

    let _ = subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));
    assert_eq!(
        subscriptions.subscribe("trades".to_string(), json!({"method":"different"})),
        Some(json!({"method":"different"}))
    );
    assert_eq!(
        subscriptions.payloads().cloned().collect::<Vec<_>>(),
        vec![json!({"method":"subscribe"}), json!({"method":"different"})]
    );

    assert_eq!(
        subscriptions.unsubscribe("trades".to_string(), json!({"method":"different"})),
        WsUnsubscribeResult::Removed {
            unsubscribe_payload: json!({"method":"unsubscribe"}),
        }
    );
    assert_eq!(
        subscriptions.payloads().cloned().collect::<Vec<_>>(),
        vec![json!({"method":"subscribe"})]
    );
}

#[test]
fn unsubscribe_waits_for_final_reference() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));
    subscriptions.subscribe("trades".to_string(), json!({"method":"subscribe"}));

    assert_eq!(
        subscriptions.unsubscribe("trades".to_string(), json!({"method":"subscribe"})),
        WsUnsubscribeResult::StillActive
    );
    assert_eq!(
        subscriptions.unsubscribe("trades".to_string(), json!({"method":"subscribe"})),
        WsUnsubscribeResult::Removed {
            unsubscribe_payload: json!({"method":"unsubscribe"}),
        }
    );
}

#[test]
fn unsubscribe_missing_topic_is_noop() {
    let mut subscriptions = ActiveWsSubscriptions::default();

    assert_eq!(
        subscriptions.unsubscribe("missing".to_string(), json!({"method":"subscribe"})),
        WsUnsubscribeResult::Missing
    );
}

#[test]
fn subscription_debug_redacts_user_topics_and_payloads() {
    let address = "0xabc0000000000000000000000000000000000000";
    let payload = json!({
        "method": "subscribe",
        "subscription": {
            "type": "openOrders",
            "user": address
        }
    });
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe(format!("openOrders:{address}"), payload.clone());

    let active_rendered = format!("{subscriptions:?}");
    assert!(active_rendered.contains("<redacted>"));
    assert!(active_rendered.contains("subscription_type: Some(\"openOrders\")"));
    assert!(!active_rendered.contains(address));

    let removed = subscriptions.unsubscribe(format!("openOrders:{address}"), payload);
    let removed_rendered = format!("{removed:?}");
    assert!(removed_rendered.contains("<redacted>"));
    assert!(!removed_rendered.contains(address));
}
