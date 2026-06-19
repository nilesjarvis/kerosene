use serde_json::json;

use super::{ActiveHydromancerSubscriptions, HydromancerUnsubscribeResult};

#[test]
fn subscribe_returns_payload_only_for_first_reference() {
    let mut subscriptions = ActiveHydromancerSubscriptions::default();

    assert_eq!(
        subscriptions.subscribe("fills".to_string(), json!({"topic":"fills"})),
        Some(json!({"topic":"fills"}))
    );
    assert_eq!(
        subscriptions.subscribe("fills".to_string(), json!({"topic":"fills"})),
        None
    );
    assert_eq!(
        subscriptions.payloads().cloned().collect::<Vec<_>>(),
        vec![json!({"topic":"fills"})]
    );
}

#[test]
fn subscribe_same_topic_with_different_payload_tracks_independent_subscription() {
    let mut subscriptions = ActiveHydromancerSubscriptions::default();

    let _ = subscriptions.subscribe("fills".to_string(), json!({"topic":"fills"}));
    assert_eq!(
        subscriptions.subscribe("fills".to_string(), json!({"topic":"different"})),
        Some(json!({"topic":"different"}))
    );
    assert_eq!(
        subscriptions.payloads().cloned().collect::<Vec<_>>(),
        vec![json!({"topic":"fills"}), json!({"topic":"different"})]
    );

    assert_eq!(
        subscriptions.unsubscribe("fills".to_string(), json!({"topic":"different"})),
        HydromancerUnsubscribeResult::Removed {
            payload: json!({"topic":"different"}),
            became_empty: false,
        }
    );
    assert_eq!(
        subscriptions.payloads().cloned().collect::<Vec<_>>(),
        vec![json!({"topic":"fills"})]
    );
}

#[test]
fn unsubscribe_tracks_reference_counts_and_final_payload() {
    let mut subscriptions = ActiveHydromancerSubscriptions::default();
    subscriptions.subscribe("fills".to_string(), json!({"topic":"fills"}));
    subscriptions.subscribe("fills".to_string(), json!({"topic":"fills"}));

    assert_eq!(
        subscriptions.unsubscribe("fills".to_string(), json!({"topic":"fills"})),
        HydromancerUnsubscribeResult::StillActive
    );
    assert_eq!(
        subscriptions.unsubscribe("fills".to_string(), json!({"topic":"fills"})),
        HydromancerUnsubscribeResult::Removed {
            payload: json!({"topic":"fills"}),
            became_empty: true,
        }
    );
    assert!(subscriptions.is_empty());
}

#[test]
fn unsubscribe_reports_when_other_topics_remain() {
    let mut subscriptions = ActiveHydromancerSubscriptions::default();
    subscriptions.subscribe("fills".to_string(), json!({"topic":"fills"}));
    subscriptions.subscribe("liquidations".to_string(), json!({"topic":"liquidations"}));

    assert_eq!(
        subscriptions.unsubscribe("fills".to_string(), json!({"topic":"fills"})),
        HydromancerUnsubscribeResult::Removed {
            payload: json!({"topic":"fills"}),
            became_empty: false,
        }
    );
    assert!(!subscriptions.is_empty());
}

#[test]
fn unsubscribe_missing_topic_is_noop() {
    let mut subscriptions = ActiveHydromancerSubscriptions::default();

    assert_eq!(
        subscriptions.unsubscribe("missing".to_string(), json!({"topic":"fills"})),
        HydromancerUnsubscribeResult::Missing
    );
}

#[test]
fn subscription_debug_redacts_tracked_trade_addresses_and_payloads() {
    let address = "0xabc0000000000000000000000000000000000000";
    let payload = json!({
        "type": "subscribe",
        "subscription": {
            "type": "userFills",
            "addresses": [address]
        }
    });
    let mut subscriptions = ActiveHydromancerSubscriptions::default();
    subscriptions.subscribe(format!("userFills:{address}"), payload.clone());

    let active_rendered = format!("{subscriptions:?}");
    assert!(active_rendered.contains("<redacted>"));
    assert!(active_rendered.contains("subscription_type: Some(\"userFills\")"));
    assert!(!active_rendered.contains(address));

    let removed = subscriptions.unsubscribe(format!("userFills:{address}"), payload);
    let removed_rendered = format!("{removed:?}");
    assert!(removed_rendered.contains("<redacted>"));
    assert!(!removed_rendered.contains(address));
}
