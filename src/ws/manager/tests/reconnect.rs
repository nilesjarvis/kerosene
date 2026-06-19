use super::*;
use serde_json::json;

#[test]
fn default_policy_holds_known_exchange_constants() {
    assert_eq!(EXCHANGE_WS_RECONNECT_POLICY.base_delay_secs, 1);
    assert_eq!(EXCHANGE_WS_RECONNECT_POLICY.max_delay_secs, 60);
    assert_eq!(EXCHANGE_WS_RECONNECT_POLICY.reset_after_secs, 30);
    assert_eq!(WS_CONNECT_TIMEOUT_SECS, 10);
    const _: () = assert!(WS_CONNECT_TIMEOUT_SECS < WS_READ_STALE_AFTER_SECS);
}

#[test]
fn next_delay_doubles_until_capped() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    assert_eq!(policy.next_delay(0), policy.base_delay_secs);
    assert_eq!(policy.next_delay(1), 2);
    assert_eq!(policy.next_delay(32), policy.max_delay_secs);
    assert_eq!(
        policy.next_delay(policy.max_delay_secs),
        policy.max_delay_secs
    );
}

#[test]
fn after_disconnect_resets_when_connection_was_stable() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let stable_for = Duration::from_secs(policy.reset_after_secs);

    let (delay, next) = policy.after_disconnect(16, stable_for);

    assert_eq!(delay, policy.base_delay_secs);
    assert_eq!(next, policy.next_delay(policy.base_delay_secs));
}

#[test]
fn after_disconnect_keeps_backing_off_after_quick_failure() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let quick = Duration::from_secs(1);

    let (delay, next) = policy.after_disconnect(8, quick);

    assert_eq!(delay, 8);
    assert_eq!(next, 16);
}

#[test]
fn policy_math_works_with_arbitrary_values() {
    // Tight policy: 1..=10s window, resets if connection survived 5s.
    let tight = ReconnectPolicy {
        base_delay_secs: 1,
        max_delay_secs: 10,
        reset_after_secs: 5,
    };

    assert_eq!(tight.next_delay(0), 1);
    assert_eq!(tight.next_delay(4), 8);
    assert_eq!(tight.next_delay(8), 10);
    assert_eq!(tight.next_delay(50), 10);

    let (delay, _) = tight.after_disconnect(8, Duration::from_secs(10));
    assert_eq!(delay, 1, "stable connection should reset to base");

    let (delay, _) = tight.after_disconnect(8, Duration::from_secs(1));
    assert_eq!(delay, 8, "unstable connection should hold backoff");
}

#[test]
fn idle_subscription_state_resets_reconnect_backoff() {
    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let empty = ActiveWsSubscriptions::default();
    let mut reconnect_delay_secs = policy.max_delay_secs;

    assert!(reset_reconnect_backoff_if_idle(
        &empty,
        &mut reconnect_delay_secs,
        policy
    ));
    assert_eq!(reconnect_delay_secs, policy.base_delay_secs);

    let mut active = ActiveWsSubscriptions::default();
    active.subscribe(
        "l2Book.BTC".to_string(),
        json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": "BTC" },
        }),
    );
    reconnect_delay_secs = policy.max_delay_secs;

    assert!(!reset_reconnect_backoff_if_idle(
        &active,
        &mut reconnect_delay_secs,
        policy
    ));
    assert_eq!(reconnect_delay_secs, policy.max_delay_secs);
}

#[test]
fn connecting_ping_does_not_restart_pending_connect() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe(
        "l2Book.BTC".to_string(),
        json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": "BTC" },
        }),
    );

    assert_eq!(
        handle_connecting_ws_command(&mut subscriptions, WsCommand::Ping),
        ConnectingWsCommandAction::ContinueConnecting
    );
    assert!(!subscriptions.is_empty());
}

#[test]
fn connecting_final_unsubscribe_restarts_pending_connect() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe(
        "l2Book.BTC".to_string(),
        json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": "BTC" },
        }),
    );

    assert_eq!(
        handle_connecting_ws_command(
            &mut subscriptions,
            WsCommand::Unsubscribe {
                topic: "l2Book.BTC".to_string(),
                payload: json!({
                    "method": "subscribe",
                    "subscription": { "type": "l2Book", "coin": "BTC" },
                }),
            },
        ),
        ConnectingWsCommandAction::RestartLoop
    );
    assert!(subscriptions.is_empty());
}

#[test]
fn connecting_nonfinal_unsubscribe_keeps_pending_connect() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    subscriptions.subscribe(
        "l2Book.BTC".to_string(),
        json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": "BTC" },
        }),
    );
    subscriptions.subscribe(
        "l2Book.BTC".to_string(),
        json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": "BTC" },
        }),
    );

    assert_eq!(
        handle_connecting_ws_command(
            &mut subscriptions,
            WsCommand::Unsubscribe {
                topic: "l2Book.BTC".to_string(),
                payload: json!({
                    "method": "subscribe",
                    "subscription": { "type": "l2Book", "coin": "BTC" },
                }),
            },
        ),
        ConnectingWsCommandAction::ContinueConnecting
    );
    assert!(!subscriptions.is_empty());
}

#[test]
fn disconnected_command_drain_updates_replay_set_without_outbound_socket() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

    cmd_tx
        .send(WsCommand::Subscribe {
            topic: "l2Book.BTC".to_string(),
            payload: json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();
    cmd_tx
        .send(WsCommand::Subscribe {
            topic: "l2Book.ETH".to_string(),
            payload: json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "ETH" },
            }),
        })
        .unwrap();
    cmd_tx
        .send(WsCommand::Unsubscribe {
            topic: "l2Book.BTC".to_string(),
            payload: json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();
    cmd_tx.send(WsCommand::Reconnect).unwrap();
    cmd_tx.send(WsCommand::Ping).unwrap();
    let reconnect_gate = WsReconnectGate::default();

    assert!(drain_disconnected_ws_commands(
        &mut subscriptions,
        &mut cmd_rx,
        &reconnect_gate,
    ));

    let replay_payloads: Vec<_> = subscriptions.payloads().cloned().collect();
    assert_eq!(replay_payloads.len(), 1);
    assert_eq!(replay_payloads[0]["subscription"]["coin"], "ETH");
}

#[tokio::test]
async fn wait_for_subscription_ignores_non_subscription_commands() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

    cmd_tx.send(WsCommand::Ping).unwrap();
    cmd_tx.send(WsCommand::Reconnect).unwrap();
    cmd_tx
        .send(WsCommand::Subscribe {
            topic: "l2Book.BTC".to_string(),
            payload: json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();
    let reconnect_gate = WsReconnectGate::default();

    let result = tokio::time::timeout(
        Duration::from_millis(50),
        wait_for_ws_subscription(&mut subscriptions, &mut cmd_rx, &reconnect_gate),
    )
    .await
    .expect("queued subscription should wake the manager");

    assert!(result);
    assert!(!subscriptions.is_empty());
}

#[tokio::test]
async fn reconnect_sleep_returns_immediately_without_subscriptions() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    let (_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let reconnect_gate = WsReconnectGate::default();

    let result = tokio::time::timeout(
        Duration::from_millis(50),
        sleep_with_disconnected_ws_commands(
            Duration::from_secs(60),
            &mut subscriptions,
            &mut cmd_rx,
            &reconnect_gate,
        ),
    )
    .await
    .expect("empty subscription set should not keep retry sleep alive");

    assert!(result);
    assert!(subscriptions.is_empty());
}

#[tokio::test]
async fn reconnect_sleep_processes_unsubscribe_before_retry() {
    let mut subscriptions = ActiveWsSubscriptions::default();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let reconnect_gate = WsReconnectGate::default();

    subscriptions.subscribe(
        "l2Book.BTC".to_string(),
        json!({
            "method": "subscribe",
            "subscription": { "type": "l2Book", "coin": "BTC" },
        }),
    );
    cmd_tx
        .send(WsCommand::Unsubscribe {
            topic: "l2Book.BTC".to_string(),
            payload: json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": "BTC" },
            }),
        })
        .unwrap();

    let result = tokio::time::timeout(
        Duration::from_millis(50),
        sleep_with_disconnected_ws_commands(
            Duration::from_secs(60),
            &mut subscriptions,
            &mut cmd_rx,
            &reconnect_gate,
        ),
    )
    .await
    .expect("queued unsubscribe should interrupt retry sleep");

    assert!(result);
    assert!(subscriptions.is_empty());
}
