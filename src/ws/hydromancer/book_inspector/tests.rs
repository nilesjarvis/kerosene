use super::*;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::sync::Arc;

fn message(kind: &str, value: serde_json::Value) -> HydromancerRoutedMessage {
    HydromancerRoutedMessage {
        msg_type: kind.into(),
        data: Arc::new(value),
    }
}

#[tokio::test]
async fn stream_filters_other_coins_and_preserves_empty_blocks() {
    let (tx, mut rx) = broadcast::channel(8);
    tx.send(message("connected", json!({}))).expect("connected");
    tx.send(message(
        "l4BookUpdates",
        json!({"coins":["ETH"], "data":{"height":10,"timestamp":10,"diffs":[]}}),
    ))
    .expect("ETH");
    tx.send(message(
        "l4BookUpdates",
        json!({"coins":["BTC"], "data":{"height":11,"timestamp":11,"diffs":[]}}),
    ))
    .expect("BTC");
    let result = next_block(&mut rx, "BTC", false).await.ok().expect("block");
    assert_eq!(result.height, 11);
    assert!(result.diffs.is_empty());
}

#[tokio::test]
async fn lagged_receivers_and_reconnects_require_resynchronization() {
    let (tx, mut rx) = broadcast::channel(1);
    for _ in 0..3 {
        tx.send(message("heartbeat", json!({}))).expect("heartbeat");
    }
    assert!(next_block(&mut rx, "BTC", true).await.is_err());
    for kind in [
        "disconnected",
        "reconnecting",
        "connected",
        "reconnected",
        "connecting",
    ] {
        assert!(control_error(&message(kind, json!({})), true).is_some());
    }
}

#[test]
fn scoped_subscription_failures_are_sanitized_and_stop_automatic_retries() {
    let error = control_error(
        &message(
            "error",
            json!({"error":"private-secret", "subscription":{"type":"l4BookUpdates"}}),
        ),
        false,
    )
    .expect("failure");
    assert!(error.permanent);
    assert!(!error.message.contains("private-secret"));
    assert!(
        control_error(
            &message("error", json!({"subscription":{"type":"candle"}})),
            true
        )
        .is_none()
    );
    assert!(
        control_error(
            &message("subscriptionResponse", json!({"success":true})),
            true
        )
        .is_none()
    );
    assert!(
        control_error(
            &message(
                "subscriptionResponse",
                json!({"failed":[{"type":"l4BookUpdates"}]})
            ),
            false
        )
        .is_some()
    );
}

#[test]
fn subscription_identity_changes_for_market_window_key_and_retry_but_not_selection() {
    let (tx, rx) = watch::channel(Selection {
        price: 100.0,
        tick: 1.0,
        is_bid: Some(true),
        page: 0,
        revision: 0,
    });
    let config = InspectorStreamConfig {
        key: HydromancerStreamKey::new("synthetic-key", 0),
        window_id: window::Id::unique(),
        run_id: 0,
        coin: "BTC".into(),
        selection: rx,
    };
    let hash = |config: &InspectorStreamConfig| {
        let mut hasher = DefaultHasher::new();
        config.hash(&mut hasher);
        hasher.finish()
    };
    let initial = hash(&config);
    tx.send_modify(|selection| {
        selection.price = 200.0;
        selection.page = 1;
        selection.revision += 1;
    });
    assert_eq!(initial, hash(&config));
    let mut changed = config.clone();
    changed.coin = "ETH".into();
    assert_ne!(initial, hash(&changed));
    let mut changed = config.clone();
    changed.run_id += 1;
    assert_ne!(initial, hash(&changed));
    let mut changed = config.clone();
    changed.window_id = window::Id::unique();
    assert_ne!(initial, hash(&changed));
    let mut changed = config;
    changed.key = HydromancerStreamKey::new("synthetic-key", 1);
    assert_ne!(initial, hash(&changed));
}

#[tokio::test]
async fn bootstrap_discards_pre_snapshot_deltas_and_applies_the_first_successor() {
    use crate::market_state::book_inspector::{RestingOrder, SCALE};
    let (tx, mut rx) = broadcast::channel(8);
    let first = Block {
        height: 9,
        timestamp_ms: 1009,
        diffs: vec![crate::market_state::book_inspector::Diff::Remove(1)],
    };
    tx.send(message("l4BookUpdates", json!({"coins":["BTC"],"data":{"height":11,"timestamp":1011,"diffs":[{"type":"update","coin":"BTC","oid":1,"sz":"0.5"}]}}))).expect("successor");
    let snapshot = async {
        Ok(Snapshot {
            height: 10,
            timestamp_ms: 1010,
            orders: vec![RestingOrder {
                oid: 1,
                wallet: "0x1111111111111111111111111111111111111111".into(),
                is_bid: true,
                price: 100 * SCALE,
                size: SCALE,
            }],
        })
    };
    let book = synchronize(&mut rx, "BTC", first, snapshot)
        .await
        .ok()
        .expect("synchronized");
    let page = book.page(Selection {
        price: 100.0,
        tick: 1.0,
        is_bid: Some(true),
        page: 0,
        revision: 0,
    });
    assert_eq!(page.height, 11);
    assert_eq!(page.orders.len(), 1);
    assert_eq!(page.orders[0].size, SCALE / 2);
}

#[tokio::test]
async fn bootstrap_rejects_a_missing_snapshot_successor() {
    let (_tx, mut rx) = broadcast::channel(8);
    let first = Block {
        height: 12,
        timestamp_ms: 1012,
        diffs: vec![],
    };
    let snapshot = async {
        Ok(Snapshot {
            height: 10,
            timestamp_ms: 1010,
            orders: vec![],
        })
    };
    let result = synchronize(&mut rx, "BTC", first, snapshot).await;
    assert!(result.err().expect("gap").message.contains("gap"));
}

#[tokio::test(start_paused = true)]
async fn bootstrap_never_publishes_a_snapshot_without_live_successor() {
    let (_tx, mut rx) = broadcast::channel(8);
    let first = Block {
        height: 10,
        timestamp_ms: 1010,
        diffs: vec![],
    };
    let snapshot = async {
        Ok(Snapshot {
            height: 10,
            timestamp_ms: 1010,
            orders: vec![],
        })
    };
    let result = synchronize(&mut rx, "BTC", first, snapshot).await;
    assert!(result.err().expect("stalled").message.contains("stalled"));
}
