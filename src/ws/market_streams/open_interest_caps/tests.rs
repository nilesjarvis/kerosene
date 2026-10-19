use super::*;
use futures::{StreamExt, channel::mpsc as output_channel};
use serde_json::json;
use std::sync::Arc;

fn fixture(time_ms: u64) -> Value {
    json!({
        "userState": {"user": CAP_STATUS_USER, "serverTime": time_ms, "agentAddress": "private-fixture"},
        "perpDexStates": [
            {"totalVaultEquity": "0", "perpsAtOpenInterestCap": ["BTC", "ETH"]},
            {"totalVaultEquity": "0"},
            {"totalVaultEquity": "0", "perpsAtOpenInterestCap": ["xyz:AAPL"]}
        ]
    })
}

fn frame(channel: &str, data: Value) -> WsRoutedMessage {
    WsRoutedMessage {
        channel: channel.to_string(),
        data: Arc::new(data),
    }
}

#[test]
fn cap_snapshot_collects_native_and_builder_markets_and_clears_removed_caps() {
    let mut value = fixture(1_000);
    let snapshot = parse_snapshot(&value, 1_100).expect("full snapshot");
    assert_eq!(
        snapshot.coins,
        HashSet::from(["BTC".to_string(), "ETH".to_string(), "xyz:AAPL".to_string()])
    );
    assert!(!format!("{snapshot:?}").contains("private-fixture"));
    value["perpDexStates"][0]
        .as_object_mut()
        .expect("state")
        .remove("perpsAtOpenInterestCap");
    value["perpDexStates"][2]["perpsAtOpenInterestCap"] = json!([]);
    assert!(
        parse_snapshot(&value, 1_100)
            .expect("empty cap lists")
            .coins
            .is_empty()
    );
}

#[test]
fn malformed_or_stale_cap_snapshots_do_not_become_all_clear() {
    for replacement in [
        json!(null),
        json!([]),
        json!([{}]),
        json!([{"totalVaultEquity":0, "perpsAtOpenInterestCap":null}]),
        json!([{"totalVaultEquity":0, "perpsAtOpenInterestCap":[42]}]),
    ] {
        let mut value = fixture(1_000);
        value["perpDexStates"] = replacement;
        assert!(parse_snapshot(&value, 1_000).is_err());
    }
    for invalid in [
        "",
        " ",
        "xyz:",
        ":BTC",
        "xyz:other:BTC",
        "@107",
        "#1",
        "PURR/USDC",
    ] {
        let mut value = fixture(1_000);
        value["perpDexStates"][0]["perpsAtOpenInterestCap"] = json!([invalid]);
        assert!(parse_snapshot(&value, 1_000).is_err());
    }
    for time in [json!(null), json!(0), json!(-1)] {
        let mut value = fixture(1_000);
        value["userState"]["serverTime"] = time;
        assert!(parse_snapshot(&value, 1_000).is_err());
    }
    assert!(parse_snapshot(&fixture(1_000), 16_000).is_err());
    assert!(parse_snapshot(&fixture(1_000), 15_999).is_ok());
}

#[tokio::test(start_paused = true)]
async fn cap_stream_routes_public_snapshots_recovers_and_unsubscribes() {
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(16);
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_cap_status(
        (WsCommandSender::new_for_test(commands), receiver),
        output,
    ));
    assert!(
        matches!(command_rx.recv().await, Some(WsCommand::Subscribe { payload, .. })
        if payload["subscription"]["type"] == "webData3" && payload["subscription"]["user"] == CAP_STATUS_USER)
    );
    let mut unrelated = fixture(crate::ws::now_ms());
    unrelated["userState"]["user"] = json!("another-account");
    frames.send(frame("webData3", unrelated)).expect("receiver");
    frames
        .send(frame("webData3", fixture(crate::ws::now_ms())))
        .expect("receiver");
    assert!(
        matches!(events.next().await, Some(OpenInterestCapStreamEvent::Snapshot(snapshot)) if snapshot.coins.contains("xyz:AAPL"))
    );

    frames
        .send(frame("keroseneDisconnected", Value::Null))
        .expect("receiver");
    assert_eq!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Unavailable)
    );
    frames
        .send(frame("webData3", fixture(crate::ws::now_ms())))
        .expect("receiver");
    assert!(matches!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Snapshot(_))
    ));

    let mut invalid = fixture(crate::ws::now_ms());
    invalid["perpDexStates"] = Value::Null;
    frames.send(frame("webData3", invalid)).expect("receiver");
    assert_eq!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Unavailable)
    );
    task.abort();
    let _ = task.await;
    assert!(
        matches!(command_rx.recv().await, Some(WsCommand::Unsubscribe { topic, .. }) if topic == CAP_STATUS_TOPIC)
    );
}

#[tokio::test(start_paused = true)]
async fn cap_stream_watchdog_expires_despite_other_market_traffic() {
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(16);
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_cap_status(
        (WsCommandSender::new_for_test(commands), receiver),
        output,
    ));
    let _ = command_rx.recv().await.expect("subscribe");
    frames
        .send(frame("webData3", fixture(crate::ws::now_ms())))
        .expect("receiver");
    assert!(matches!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Snapshot(_))
    ));
    tokio::time::advance(Duration::from_secs(14)).await;
    frames
        .send(frame("activeAssetCtx", json!({})))
        .expect("receiver");
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Unavailable)
    );
    assert!(
        matches!(command_rx.recv().await, Some(WsCommand::Resubscribe { topic }) if topic == CAP_STATUS_TOPIC)
    );
    frames
        .send(frame("webData3", fixture(crate::ws::now_ms())))
        .expect("receiver");
    assert!(matches!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Snapshot(_))
    ));
    drop(frames);
    assert_eq!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Unavailable)
    );
    task.await.expect("closed stream");
}

#[tokio::test(start_paused = true)]
async fn cap_stream_lag_invalidates_status_and_requests_reconnect() {
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(1);
    for _ in 0..3 {
        frames
            .send(frame("unrelated", Value::Null))
            .expect("receiver");
    }
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_cap_status(
        (WsCommandSender::new_for_test(commands), receiver),
        output,
    ));
    assert!(matches!(
        command_rx.recv().await,
        Some(WsCommand::Subscribe { .. })
    ));
    assert_eq!(
        events.next().await,
        Some(OpenInterestCapStreamEvent::Unavailable)
    );
    assert!(matches!(
        command_rx.recv().await,
        Some(WsCommand::Reconnect)
    ));
    task.abort();
    let _ = task.await;
}
