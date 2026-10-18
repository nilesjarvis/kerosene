use super::*;
use serde_json::json;

const BUYER: &str = "0x1111111111111111111111111111111111111111";
const SELLER: &str = "0x2222222222222222222222222222222222222222";

fn fixture() -> Value {
    json!({"coin":"xyz:BTC", "side":"B", "px":"123.456", "sz":"0.012", "time": 1234, "tid": 42, "users":[BUYER, SELLER]})
}

#[test]
fn trade_parser_preserves_precision_and_selects_initiating_trader() {
    let trade = parse_trade(&fixture(), "xyz:BTC").expect("valid trade");
    assert!(trade.is_buy);
    assert_eq!(trade.price, "123.456");
    assert_eq!(trade.size, "0.012");
    assert!((trade.value - 1.481472).abs() < 1e-9);
    assert_eq!(trade.trader.as_deref(), Some(BUYER));
    assert!(parse_trade(&fixture(), "BTC").is_none());
    let mut sell = fixture();
    sell["side"] = json!("A");
    let trade = parse_trade(&sell, "xyz:BTC").expect("valid sell");
    assert!(!trade.is_buy);
    assert_eq!(trade.trader.as_deref(), Some(SELLER));
}

#[test]
fn trade_feed_missing_or_invalid_traders_do_not_drop_executions() {
    for users in [
        Value::Null,
        json!([]),
        json!([BUYER]),
        json!([BUYER, SELLER, BUYER]),
        json!([null, SELLER]),
        json!([42, SELLER]),
        json!(["invalid-address", SELLER]),
        json!([format!("0x{}", "g".repeat(40)), SELLER]),
        json!(["a".repeat(100_000), SELLER]),
    ] {
        let mut value = fixture();
        value["users"] = users;
        let trade = parse_trade(&value, "xyz:BTC").expect("execution remains visible");
        assert!(trade.trader.is_none());
    }
    let mut value = fixture();
    value.as_object_mut().expect("object").remove("users");
    assert!(
        parse_trade(&value, "xyz:BTC")
            .expect("no users")
            .trader
            .is_none()
    );
    value["users"] = json!(["0xABCDEFabcdefABCDEFabcdefABCDEFabcdefABCD", SELLER]);
    assert_eq!(
        parse_trade(&value, "xyz:BTC")
            .expect("mixed case")
            .trader
            .as_deref(),
        Some("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd")
    );
}

#[test]
fn trade_feed_stream_debug_redacts_trader_addresses() {
    let mut sell = fixture();
    sell["side"] = json!("A");
    let message = crate::message::Message::TradeFeedStream(
        9,
        3,
        "xyz:BTC".into(),
        TradeFeedEvent::Trades(vec![
            parse_trade(&fixture(), "xyz:BTC").expect("buy"),
            parse_trade(&sell, "xyz:BTC").expect("sell"),
        ]),
    );
    let debug = format!("{message:?}");
    assert!(!debug.contains(BUYER));
    assert!(!debug.contains(SELLER));
    assert!(debug.contains("Address(<redacted>)"));
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
