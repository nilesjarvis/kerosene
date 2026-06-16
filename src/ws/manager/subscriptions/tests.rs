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
