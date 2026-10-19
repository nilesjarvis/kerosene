use super::*;
use futures::{StreamExt, channel::mpsc as output_channel};
use serde_json::json;
use std::sync::Arc;

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";
fn fill(sequence: u64) -> Value {
    json!([ADDRESS, {"coin":"BTC", "side":"B", "px":"123.456", "sz":"0.012",
        "time": 1000 + sequence, "tid":sequence, "crossed":true,
        "startPosition":"0", "dir":"Open Long"}])
}
fn message(kind: &str, value: Value) -> HydromancerRoutedMessage {
    HydromancerRoutedMessage {
        msg_type: kind.into(),
        data: Arc::new(value),
    }
}

#[test]
fn trade_feed_hydromancer_subscriptions_always_filter_coins_and_disable_server_aggregation() {
    assert!(subscription("").is_none());
    assert!(subscription("  ").is_none());
    for coin in ["BTC", "xyz:SP500", "@107", "#123"] {
        let (_, payload) = subscription(coin).expect("filtered subscription");
        assert_eq!(
            payload["subscription"],
            json!({"type":"allFills", "coins":[coin], "aggregateByTime":false})
        );
    }
}

#[test]
fn trade_feed_hydromancer_parser_keeps_only_taker_and_preserves_precision_and_address_privacy() {
    let raw = fill(1);
    let trade = parse_fill(&raw, "BTC").expect("taker");
    assert_eq!(trade.trader.as_deref(), Some(ADDRESS));
    assert_eq!(trade.price, "123.456");
    assert_eq!(trade.size, "0.012");
    assert_eq!(trade.intent, TrackedTradeIntent::Opening);
    assert!(!format!("{trade:?}").contains(ADDRESS));
    assert!(parse_fill(&raw, "ETH").is_none());
    for crossed in [json!(false), Value::Null, json!("true")] {
        let mut raw = fill(1);
        raw[1]["crossed"] = crossed;
        assert!(parse_fill(&raw, "BTC").is_none());
    }
    let mut raw = fill(1);
    raw[0] = Value::Null;
    assert!(
        parse_fill(&raw, "BTC")
            .expect("missing trader still shows print")
            .trader
            .is_none()
    );
    raw[1]["px"] = json!("NaN");
    assert!(parse_fill(&raw, "BTC").is_none());
}

#[test]
fn trade_feed_hydromancer_parser_classifies_positions_and_leaves_missing_context_unknown() {
    use TrackedTradeIntent::*;
    for (start, side, size, expected) in [
        ("0", "B", "1", Opening),
        ("2", "B", "1", Increasing),
        ("2", "A", "1", Reducing),
        ("2", "A", "2", Closing),
        ("2", "A", "3", Reversing),
        ("-2", "A", "1", Increasing),
        ("-2", "B", "1", Reducing),
        ("-2", "B", "2", Closing),
        ("-2", "B", "3", Reversing),
        ("NaN", "B", "1", Unknown),
    ] {
        let mut raw = fill(1);
        raw[1]["startPosition"] = json!(start);
        raw[1]["side"] = json!(side);
        raw[1]["sz"] = json!(size);
        assert_eq!(parse_fill(&raw, "BTC").expect("fill").intent, expected);
    }
    let mut raw = fill(1);
    raw[1]["startPosition"] = Value::Null;
    assert_eq!(
        parse_fill(&raw, "BTC").expect("no position").intent,
        Unknown
    );
    raw = fill(1);
    raw[1]["dir"] = json!("Buy");
    assert_eq!(
        parse_fill(&raw, "BTC").expect("unknown direction").intent,
        Unknown
    );
    for coin in ["@107", "#123", "PURR/USDC"] {
        raw = fill(1);
        raw[1]["coin"] = json!(coin);
        assert_eq!(parse_fill(&raw, coin).expect("non-perp").intent, Unknown);
    }
}

#[test]
fn trade_feed_hydromancer_controls_scope_failures_to_the_correct_asset() {
    for value in [
        json!({"type":"error","subscription":{"type":"allFills","coins":["ETH"]},"error":"denied"}),
        json!({"subscription":{"type":"userFills"},"error":"denied"}),
        json!({"operation":"unsubscribe","subscription":{"type":"allFills","coins":["BTC"]}}),
        json!({"failed":[{"subscription":{"type":"allFills","coins":["ETH"]},"error":"denied"}]}),
    ] {
        assert!(control_event(&message("subscriptionUpdate", value), "BTC").is_none());
    }
    for value in [
        json!({"subscription":{"type":"allFills","coins":["BTC"]},"failed":["allFills"]}),
        json!({"subscription":{"type":"allFills","coins":["BTC"]},"success":false}),
        json!({"failed":[{"type":"allFills","coins":["BTC"],"error":"quota"}]}),
    ] {
        assert!(matches!(
            control_event(&message("subscriptionUpdate", value), "BTC"),
            Some(TradeFeedEvent::ContextUnavailable)
        ));
    }
    assert!(matches!(
        control_event(&message("error", json!({"message":"secret-key"})), "BTC"),
        Some(TradeFeedEvent::ContextUnavailable)
    ));
    assert!(matches!(
        control_event(
            &message(
                "subscriptionUpdate",
                json!({"subscription":{"type":"allFills","coins":["BTC"]},"subscribed":["allFills"]})
            ),
            "BTC"
        ),
        Some(TradeFeedEvent::Subscribed)
    ));
}

#[tokio::test(start_paused = true)]
async fn trade_feed_hydromancer_batches_all_fills_and_deduplicates_replays_in_the_model() {
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(16);
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_trade_feed(
        (7, 2, "BTC".into()),
        (HydromancerCommandSender::new_for_test(commands), receiver),
        output,
    ));
    assert!(
        matches!(command_rx.recv().await, Some(HydromancerCommand::Subscribe { payload, .. }) if payload["subscription"]["coins"] == json!(["BTC"]))
    );
    let mut maker = fill(9999);
    maker[1]["crossed"] = json!(false);
    let mut items: Vec<_> = (1..=600).map(fill).collect();
    items.push(maker);
    frames
        .send(message(
            "allFills",
            json!({"type":"allFills","fills":items}),
        ))
        .expect("send fills");
    let mut feed = crate::trade_feed_state::TradeFeedInstance::new(Default::default());
    let mut received = 0;
    tokio::time::timeout(Duration::from_secs(1), async {
        while received < 600 {
            let (id, generation, coin, event) = events.next().await.expect("batch");
            assert_eq!((id, generation, coin.as_str()), (7, 2, "BTC"));
            if let TradeFeedEvent::Trades(ref trades) = event {
                received += trades.len();
            }
            feed.apply(event);
        }
    })
    .await
    .expect("all fills delivered");
    assert_eq!(received, 600);
    assert_eq!(feed.trades.len(), 500);
    frames
        .send(message(
            "replay",
            json!({"type":"replay","channel":"allFills","data":[fill(600), fill(601)]}),
        ))
        .expect("replay");
    let (_, _, _, event) = events.next().await.expect("replayed batch");
    feed.apply(event);
    assert_eq!(feed.trades.len(), 500);
    assert_eq!(feed.trades[0].tid, 601);
    assert_eq!(
        feed.trades.iter().filter(|trade| trade.tid == 600).count(),
        1
    );
    task.abort();
    let _ = task.await;
    assert!(
        matches!(command_rx.recv().await, Some(HydromancerCommand::Unsubscribe { payload, .. }) if payload["subscription"]["coins"] == json!(["BTC"]))
    );
}

#[tokio::test(start_paused = true)]
async fn trade_feed_hydromancer_unacknowledged_subscription_times_out_and_releases_capacity() {
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (_frames, receiver) = broadcast::channel(8);
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_trade_feed(
        (7, 2, "BTC".into()),
        (HydromancerCommandSender::new_for_test(commands), receiver),
        output,
    ));
    command_rx.recv().await.expect("subscribe");
    tokio::time::advance(SUBSCRIBE_TIMEOUT).await;
    let (_, _, _, event) = events.next().await.expect("fallback");
    assert!(matches!(event, TradeFeedEvent::ContextUnavailable));
    task.await.expect("stream finished");
    assert!(matches!(
        command_rx.recv().await,
        Some(HydromancerCommand::Unsubscribe { .. })
    ));
}

#[tokio::test(start_paused = true)]
async fn trade_feed_hydromancer_acknowledged_quiet_market_stays_active_then_disconnect_falls_back()
{
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(8);
    let (output, mut events) = output_channel::unbounded();
    let task = tokio::spawn(forward_trade_feed(
        (7, 2, "BTC".into()),
        (HydromancerCommandSender::new_for_test(commands), receiver),
        output,
    ));
    command_rx.recv().await.expect("subscribe");
    frames
        .send(message(
            "subscriptionUpdate",
            json!({"subscription":{"type":"allFills","coins":["BTC"]},"subscribed":["allFills"]}),
        ))
        .expect("ack");
    assert!(matches!(
        events.next().await.expect("subscribed").3,
        TradeFeedEvent::Subscribed
    ));
    tokio::time::advance(SUBSCRIBE_TIMEOUT * 2).await;
    assert!(!task.is_finished());
    frames
        .send(message(
            "disconnected",
            json!({"message":"synthetic-secret-key"}),
        ))
        .expect("disconnect");
    let event = events.next().await.expect("fallback").3;
    assert!(matches!(event, TradeFeedEvent::ContextUnavailable));
    assert!(!format!("{event:?}").contains("synthetic-secret-key"));
    task.await.expect("stream finished");
    assert!(matches!(
        command_rx.recv().await,
        Some(HydromancerCommand::Unsubscribe { .. })
    ));
}

#[tokio::test]
async fn trade_feed_hydromancer_lag_falls_back_and_unsubscribes() {
    let (commands, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (frames, receiver) = broadcast::channel(1);
    let (output, mut events) = output_channel::unbounded();
    for _ in 0..3 {
        frames
            .send(message("heartbeat", json!({})))
            .expect("queued");
    }
    forward_trade_feed(
        (7, 2, "BTC".into()),
        (HydromancerCommandSender::new_for_test(commands), receiver),
        output,
    )
    .await;
    assert!(matches!(
        events.next().await.expect("fallback").3,
        TradeFeedEvent::ContextUnavailable
    ));
    assert!(matches!(
        command_rx.try_recv(),
        Ok(HydromancerCommand::Subscribe { .. })
    ));
    assert!(matches!(
        command_rx.try_recv(),
        Ok(HydromancerCommand::Unsubscribe { .. })
    ));
}
