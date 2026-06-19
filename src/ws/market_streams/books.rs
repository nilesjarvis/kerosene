use super::{KeyedBookStreamEvent, WsStreamEvent};
use crate::api::{OrderBook, parse_ws_book};
use crate::ws::{
    L2BookSigfigs, SubscriptionGuard, WsCommand, get_manager, l2_book_payload_matches_sigfigs,
};

use futures::SinkExt as _;
use std::pin::Pin;
use tokio::sync::broadcast;

type BookSigfigs = L2BookSigfigs;
type BookEventStream =
    Pin<Box<dyn futures::Stream<Item = WsStreamEvent<(String, OrderBook)>> + Send>>;
type KeyedBookEventStream = Pin<Box<dyn futures::Stream<Item = KeyedBookStreamEvent> + Send>>;

// ---------------------------------------------------------------------------
// Order Book Streams
// ---------------------------------------------------------------------------

fn ws_book_event_stream(coin: &str, sigfigs: BookSigfigs) -> BookEventStream {
    let coin = coin.to_string();

    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_manager();

        let topic = format!(
            "l2Book:{}:{}:{}",
            coin,
            sigfigs.0.unwrap_or(0),
            sigfigs.1.unwrap_or(0)
        );
        let mut sub_payload = serde_json::json!({
            "type": "l2Book",
            "coin": coin,
        });
        if let Some(subscription) = sub_payload.as_object_mut() {
            if let Some(n) = sigfigs.0 {
                subscription.insert("nSigFigs".to_string(), serde_json::json!(n));
            }
            if let Some(m) = sigfigs.1 {
                subscription.insert("mantissa".to_string(), serde_json::json!(m));
            }
        }
        let payload = serde_json::json!({
            "method": "subscribe",
            "subscription": sub_payload
        });
        let subscription = (topic.clone(), payload.clone());

        if cmd_tx
            .send(WsCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let reconnect_tx = cmd_tx.clone();
        let _guard = SubscriptionGuard {
            cmd_tx,
            subscriptions: vec![subscription],
        };

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if msg.channel == "l2Book"
                        && msg.data.get("coin").and_then(|v| v.as_str()) == Some(&coin)
                        && l2_book_payload_matches_sigfigs(&msg.data, sigfigs)
                        && let Some(book) = parse_ws_book(&msg.data)
                        && output
                            .send(WsStreamEvent::Item((coin.clone(), book)))
                            .await
                            .is_err()
                    {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    if !super::emit_lag_after_reconnect(
                        &reconnect_tx,
                        WsStreamEvent::Lagged { skipped },
                        |event| async { output.send(event).await.is_ok() },
                        std::time::Duration::from_secs(super::WS_LAG_RECONNECT_PAUSE_SECS),
                    )
                    .await
                    {
                        return;
                    }
                }
                Err(error) if crate::ws::broadcast_receiver_closed(&error) => {
                    return;
                }
                Err(_error) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }))
}

pub fn ws_book_stream_keyed_events(params: &(u64, String, BookSigfigs)) -> KeyedBookEventStream {
    let book_id = params.0;
    let coin = params.1.clone();
    let sigfigs = params.2;
    let inner = ws_book_event_stream(&params.1, params.2);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item((coin, book)) => {
            KeyedBookStreamEvent::Item(book_id, coin, sigfigs, None, book)
        }
        WsStreamEvent::Lagged { skipped } => KeyedBookStreamEvent::Lagged {
            id: book_id,
            coin: coin.clone(),
            sigfigs,
            hydromancer_key_generation: None,
            skipped,
        },
    }))
}
