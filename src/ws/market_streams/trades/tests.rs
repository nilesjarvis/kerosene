use super::*;
use serde_json::json;

fn fixture() -> Value {
    json!({"coin":"xyz:BTC", "side":"B", "px":"123.456", "sz":"0.012", "time": 1234, "tid": 42, "users":["private-buyer", "private-seller"]})
}

#[test]
fn trade_parser_preserves_precision_and_omits_participants() {
    let trade = parse_trade(&fixture(), "xyz:BTC").expect("valid trade");
    assert!(trade.is_buy);
    assert_eq!(trade.price, "123.456");
    assert_eq!(trade.size, "0.012");
    assert!((trade.value - 1.481472).abs() < 1e-9);
    assert!(!format!("{trade:?}").contains("private-"));
    assert!(parse_trade(&fixture(), "BTC").is_none());
}

#[test]
fn trade_parser_rejects_invalid_numbers_and_unknown_sides() {
    for (key, invalid) in [
        ("px", json!("NaN")),
        ("sz", json!("inf")),
        ("px", json!("0")),
        ("sz", json!("-1")),
        ("side", json!("unknown")),
        ("time", json!(0)),
        ("tid", json!(-1)),
    ] {
        let mut value = fixture();
        value[key] = invalid;
        assert!(
            parse_trade(&value, "xyz:BTC").is_none(),
            "accepted invalid {key}"
        );
    }
    let mut value = fixture();
    value["px"] = json!("1e300");
    value["sz"] = json!("1e300");
    assert!(parse_trade(&value, "xyz:BTC").is_none());
    value = fixture();
    value["side"] = json!("A");
    assert!(!parse_trade(&value, "xyz:BTC").expect("sell").is_buy);
}

#[tokio::test(start_paused = true)]
async fn trade_feed_batches_every_execution_and_unsubscribes_when_cancelled() {
    use futures::{StreamExt, channel::mpsc as output_channel};
    use std::sync::Arc;
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(16);
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_trade_feed(
        (9, 3, "xyz:BTC".into()),
        (WsCommandSender::new_for_test(commands), receiver),
        output,
    ));
    let subscribe = command_rx.recv().await.expect("subscribe");
    assert!(
        matches!(subscribe, WsCommand::Subscribe { topic, payload } if topic == "trades:xyz:BTC" && payload["subscription"]["coin"] == "xyz:BTC")
    );
    let mut trades: Vec<_> = (0..1200)
        .map(|tid| {
            let mut trade = fixture();
            trade["tid"] = json!(tid);
            trade
        })
        .collect();
    let mut other = fixture();
    other["coin"] = json!("BTC");
    trades.push(other);
    frames
        .send(WsRoutedMessage {
            channel: "trades".into(),
            data: Arc::new(json!(trades)),
        })
        .expect("receiver");
    let mut ids = Vec::new();
    while ids.len() < 1200 {
        let (id, generation, coin, event) = events.next().await.expect("batch");
        assert_eq!((id, generation, coin.as_str()), (9, 3, "xyz:BTC"));
        let TradeFeedEvent::Trades(trades) = event else {
            panic!("expected trades");
        };
        assert!(trades.len() <= BATCH_CAPACITY);
        ids.extend(trades.into_iter().map(|trade| trade.tid));
    }
    assert_eq!(ids, (0..1200).collect::<Vec<_>>());
    task.abort();
    let _ = task.await;
    assert!(
        matches!(command_rx.recv().await, Some(WsCommand::Unsubscribe { topic, .. }) if topic == "trades:xyz:BTC")
    );
}

#[tokio::test(start_paused = true)]
async fn trade_feed_lag_reports_a_gap_and_requests_reconnect() {
    use futures::{StreamExt, channel::mpsc as output_channel};
    use std::sync::Arc;
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(1);
    for _ in 0..3 {
        frames
            .send(WsRoutedMessage {
                channel: "unrelated".into(),
                data: Arc::new(Value::Null),
            })
            .expect("receiver");
    }
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_trade_feed(
        (9, 3, "xyz:BTC".into()),
        (WsCommandSender::new_for_test(commands), receiver),
        output,
    ));
    assert!(matches!(
        command_rx.recv().await,
        Some(WsCommand::Subscribe { .. })
    ));
    assert!(matches!(
        events.next().await,
        Some((9, 3, _, TradeFeedEvent::Lagged))
    ));
    assert!(matches!(
        command_rx.recv().await,
        Some(WsCommand::Reconnect)
    ));
    task.abort();
    let _ = task.await;
}
