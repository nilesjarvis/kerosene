use super::*;
use futures::{StreamExt as _, channel::mpsc as output_channel};
use serde_json::json;
use std::{sync::Arc, task::Poll};
use tokio::sync::mpsc;

const FEEDS: [(FillFeed, &str); 2] = [
    (FillFeed::Liquidations, "liquidationFills"),
    (FillFeed::TrackedTrades, "userFills"),
];

fn subscription() -> (String, Value) {
    (
        "fixture-topic".to_string(),
        json!({ "subscription": "fixture" }),
    )
}

fn routed(msg_type: &str, data: Value) -> HydromancerRoutedMessage {
    HydromancerRoutedMessage {
        msg_type: msg_type.to_string(),
        data: Arc::new(data),
    }
}

fn fill(sequence: u64) -> Value {
    json!([
        "synthetic-address",
        {
            "coin": "BTC", "px": "10.5", "sz": "2", "side": "B",
            "time": sequence, "txIndex": sequence, "tid": sequence,
            "closedPnl": "0", "fee": "0.01",
            "liquidation": { "method": "market", "liquidatedUser": "synthetic-user" }
        }
    ])
}

fn assert_fill(feed: FillFeed, event: HydromancerWsMessage, sequence: u64) {
    match (feed, event) {
        (FillFeed::Liquidations, HydromancerWsMessage::Event(event)) => {
            assert_eq!(event.time_ms, sequence);
            assert_eq!(event.liquidated_user, "synthetic-user");
        }
        (FillFeed::TrackedTrades, HydromancerWsMessage::TrackedTrade(event)) => {
            assert_eq!(event.time_ms, sequence);
            assert_eq!(event.address, "synthetic-address");
        }
        _ => panic!("fill should retain its feed-specific event variant"),
    }
}

fn assert_subscribe(commands: &mut mpsc::UnboundedReceiver<HydromancerCommand>) {
    let HydromancerCommand::Subscribe { topic, payload } = commands
        .try_recv()
        .expect("subscription should be sent first")
    else {
        panic!("expected subscribe");
    };
    assert_eq!((topic, payload), subscription());
}

fn assert_unsubscribe(commands: &mut mpsc::UnboundedReceiver<HydromancerCommand>) {
    let HydromancerCommand::Unsubscribe { topic, payload } = commands
        .try_recv()
        .expect("subscription should be released")
    else {
        panic!("expected unsubscribe");
    };
    assert_eq!((topic, payload), subscription());
    assert!(
        commands.try_recv().is_err(),
        "no other command should be sent"
    );
}

#[tokio::test]
async fn fill_streams_forward_controls_and_deduplicate_live_and_replayed_events() {
    for (feed, channel) in FEEDS {
        let (cmd_tx, mut commands) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(16);
        let (output, results) = output_channel::unbounded();
        for message in [
            // Control forwarding must not skip data from the same routed message.
            routed(
                "connected",
                json!({ "type": channel, "fills": [fill(1), null] }),
            ),
            routed(
                "replay",
                json!({ "type": "replay", "channel": "other", "data": [fill(9)] }),
            ),
            routed(
                "replay",
                json!({ "type": "replay", "channel": channel, "data": [fill(1), fill(2)] }),
            ),
            routed(
                "reconnecting",
                json!({ "error": "network unavailable", "retryDelaySecs": 7 }),
            ),
            routed(
                channel,
                json!({ "type": channel, "fills": [fill(1), fill(2), fill(3)] }),
            ),
        ] {
            msg_tx.send(message).expect("feed receiver should be open");
        }
        drop(msg_tx);

        forward_fill_stream(
            feed,
            subscription(),
            (HydromancerCommandSender::new_for_test(cmd_tx), msg_rx),
            output,
        )
        .await;

        let mut events = results.collect::<Vec<_>>().await.into_iter();
        assert!(matches!(
            events.next(),
            Some(HydromancerWsMessage::Connected)
        ));
        assert_fill(feed, events.next().expect("first fill"), 1);
        assert_fill(feed, events.next().expect("replayed fill"), 2);
        let Some(HydromancerWsMessage::Reconnecting {
            error,
            retry_delay_secs,
        }) = events.next()
        else {
            panic!("reconnect status should retain its position between fills");
        };
        assert_eq!(error, "network unavailable");
        assert_eq!(retry_delay_secs, 7);
        assert_fill(feed, events.next().expect("fill after reconnect"), 3);
        assert!(events.next().is_none());
        assert_subscribe(&mut commands);
        assert_unsubscribe(&mut commands);
    }
}

#[tokio::test]
async fn failed_control_or_fill_delivery_unsubscribes() {
    for (feed, channel) in FEEDS {
        for message in [
            routed("connected", json!({})),
            routed(channel, json!({ "type": channel, "fills": [fill(1)] })),
        ] {
            let (cmd_tx, mut commands) = mpsc::unbounded_channel();
            let (msg_tx, msg_rx) = broadcast::channel(8);
            let (output, results) = output_channel::unbounded();
            drop(results);
            msg_tx.send(message).expect("feed receiver should be open");

            tokio::time::timeout(
                Duration::from_secs(1),
                forward_fill_stream(
                    feed,
                    subscription(),
                    (HydromancerCommandSender::new_for_test(cmd_tx), msg_rx),
                    output,
                ),
            )
            .await
            .expect("failed delivery should stop the stream");

            assert_subscribe(&mut commands);
            assert_unsubscribe(&mut commands);
            assert_eq!(msg_tx.receiver_count(), 0);
        }
    }
}

#[tokio::test]
async fn lag_requests_reconnect_before_failed_delivery_and_unsubscribe() {
    for (feed, _) in FEEDS {
        let (cmd_tx, mut commands) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(1);
        let (output, results) = output_channel::unbounded();
        drop(results);
        for _ in 0..2 {
            msg_tx
                .send(routed("heartbeat", json!({})))
                .expect("open receiver");
        }

        tokio::time::timeout(
            Duration::from_secs(1),
            forward_fill_stream(
                feed,
                subscription(),
                (HydromancerCommandSender::new_for_test(cmd_tx), msg_rx),
                output,
            ),
        )
        .await
        .expect("failed lag delivery should stop without the reconnect pause");

        assert_subscribe(&mut commands);
        assert!(matches!(
            commands.try_recv(),
            Ok(HydromancerCommand::Reconnect)
        ));
        assert_unsubscribe(&mut commands);
    }
}

#[tokio::test]
async fn lag_delivery_pauses_before_reading_more_frames() {
    for (feed, _) in FEEDS {
        let (cmd_tx, mut commands) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(1);
        let (output, mut results) = output_channel::unbounded();
        for _ in 0..2 {
            msg_tx
                .send(routed("heartbeat", json!({})))
                .expect("open receiver");
        }
        let mut forwarding = Box::pin(forward_fill_stream(
            feed,
            subscription(),
            (HydromancerCommandSender::new_for_test(cmd_tx), msg_rx),
            output,
        ));

        assert!(matches!(futures::poll!(forwarding.as_mut()), Poll::Pending));
        assert_subscribe(&mut commands);
        assert!(matches!(
            commands.try_recv(),
            Ok(HydromancerCommand::Reconnect)
        ));
        assert!(matches!(
            futures::poll!(results.next()),
            Poll::Ready(Some(HydromancerWsMessage::Lagged { skipped: 1 }))
        ));
        assert!(matches!(futures::poll!(forwarding.as_mut()), Poll::Pending));
        assert!(matches!(futures::poll!(results.next()), Poll::Pending));

        drop(forwarding);
        assert_unsubscribe(&mut commands);
    }
}

#[tokio::test]
async fn dropping_an_idle_fill_stream_unsubscribes() {
    for (feed, _) in FEEDS {
        let (cmd_tx, mut commands) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(1);
        let (output, _results) = output_channel::unbounded();
        let mut forwarding = Box::pin(forward_fill_stream(
            feed,
            subscription(),
            (HydromancerCommandSender::new_for_test(cmd_tx), msg_rx),
            output,
        ));

        assert!(
            commands.try_recv().is_err(),
            "setup should wait for the first poll"
        );
        assert!(matches!(futures::poll!(forwarding.as_mut()), Poll::Pending));
        assert_subscribe(&mut commands);
        drop(forwarding);
        assert_unsubscribe(&mut commands);
        assert_eq!(msg_tx.receiver_count(), 0);
    }
}

#[tokio::test]
async fn failed_subscription_stops_before_forwarding() {
    for (feed, _) in FEEDS {
        let (cmd_tx, commands) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(1);
        let (output, results) = output_channel::unbounded();
        drop(commands);
        msg_tx
            .send(routed("connected", json!({})))
            .expect("open receiver");

        forward_fill_stream(
            feed,
            subscription(),
            (HydromancerCommandSender::new_for_test(cmd_tx), msg_rx),
            output,
        )
        .await;

        assert!(results.collect::<Vec<_>>().await.is_empty());
        assert_eq!(msg_tx.receiver_count(), 0);
    }
}
