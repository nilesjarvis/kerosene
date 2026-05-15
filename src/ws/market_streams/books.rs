use crate::api::{OrderBook, parse_ws_book};
use crate::ws::{SubscriptionGuard, WsCommand, get_manager};

use futures::SinkExt as _;
use std::pin::Pin;
use tokio::sync::broadcast;

type BookSigfigs = (Option<u8>, Option<u8>);
type BookStream = Pin<Box<dyn futures::Stream<Item = (String, OrderBook)> + Send>>;
type KeyedBookStream =
    Pin<Box<dyn futures::Stream<Item = (u64, String, BookSigfigs, OrderBook)> + Send>>;

// ---------------------------------------------------------------------------
// Order Book Streams
// ---------------------------------------------------------------------------

fn ws_book_stream(coin: &str, sigfigs: BookSigfigs) -> BookStream {
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

        if cmd_tx
            .send(WsCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let _guard = SubscriptionGuard {
            cmd_tx,
            topics: vec![topic],
        };

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if msg.channel == "l2Book"
                        && msg.data.get("coin").and_then(|v| v.as_str()) == Some(&coin)
                        && let Some(book) = parse_ws_book(&msg.data)
                        && output.send((coin.clone(), book)).await.is_err()
                    {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }))
}

pub fn ws_book_stream_keyed(params: &(u64, String, BookSigfigs)) -> KeyedBookStream {
    let book_id = params.0;
    let sigfigs = params.2;
    let inner = ws_book_stream(&params.1, params.2);
    Box::pin(futures::StreamExt::map(inner, move |(coin, book)| {
        (book_id, coin, sigfigs, book)
    }))
}
