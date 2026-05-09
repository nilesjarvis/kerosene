use crate::api::Candle;
use crate::ws::{SubscriptionGuard, WsCommand, WsStream, get_manager};

use super::KeyedCandleUpdate;
use futures::SinkExt as _;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Candle Streams
// ---------------------------------------------------------------------------

fn ws_candle_stream(
    params: &(String, String),
) -> std::pin::Pin<Box<dyn futures::Stream<Item = Candle> + Send>> {
    let coin = params.0.clone();
    let interval = params.1.clone();

    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_manager();

        let topic = format!("candle:{}:{}", coin, interval);
        let payload = serde_json::json!({
            "method": "subscribe",
            "subscription": {
                "type": "candle",
                "coin": coin,
                "interval": interval,
            }
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
                    if msg.channel == "candle"
                        && msg.data.get("s").and_then(|v| v.as_str()) == Some(&coin)
                        && msg.data.get("i").and_then(|v| v.as_str()) == Some(&interval)
                        && let Ok(candle) = serde_json::from_value::<Candle>((*msg.data).clone())
                        && output.send(candle).await.is_err()
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

pub fn ws_candle_stream_keyed(params: &(u64, String, String)) -> WsStream<KeyedCandleUpdate> {
    let chart_id = params.0;
    let coin = params.1.clone();
    let interval = params.2.clone();
    let pair = (params.1.clone(), params.2.clone());
    let inner = ws_candle_stream(&pair);
    Box::pin(futures::StreamExt::map(inner, move |candle| {
        (chart_id, coin.clone(), interval.clone(), candle)
    }))
}

pub fn ws_spaghetti_candle_stream(
    params: &(u64, String, String),
) -> std::pin::Pin<Box<dyn futures::Stream<Item = (u64, String, Candle)> + Send>> {
    let spaghetti_id = params.0;
    let coin = params.1.clone();
    let pair = (params.1.clone(), params.2.clone());
    let inner = ws_candle_stream(&pair);
    Box::pin(futures::StreamExt::map(inner, move |candle| {
        (spaghetti_id, coin.clone(), candle)
    }))
}
