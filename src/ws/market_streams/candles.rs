use crate::api::Candle;
use crate::ws::{SubscriptionGuard, WsCommand, WsStream, get_manager};

use super::{KeyedCandleStreamEvent, SpaghettiCandleStreamEvent, WsStreamEvent};
use futures::SinkExt as _;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Candle Streams
// ---------------------------------------------------------------------------

fn ws_candle_stream(params: &(String, String)) -> WsStream<WsStreamEvent<Candle>> {
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
                    if msg.channel == "candle"
                        && msg.data.get("s").and_then(|v| v.as_str()) == Some(&coin)
                        && msg.data.get("i").and_then(|v| v.as_str()) == Some(&interval)
                        && let Ok(candle) = serde_json::from_value::<Candle>((*msg.data).clone())
                        && output.send(WsStreamEvent::Item(candle)).await.is_err()
                    {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    if output
                        .send(WsStreamEvent::Lagged { skipped })
                        .await
                        .is_err()
                    {
                        return;
                    }
                    if !super::request_ws_reconnect_after_lag(&reconnect_tx) {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(
                        super::WS_LAG_RECONNECT_PAUSE_SECS,
                    ))
                    .await;
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

pub fn ws_candle_stream_keyed(params: &(u64, String, String)) -> WsStream<KeyedCandleStreamEvent> {
    let chart_id = params.0;
    let coin = params.1.clone();
    let interval = params.2.clone();
    let pair = (params.1.clone(), params.2.clone());
    let inner = ws_candle_stream(&pair);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item(candle) => {
            KeyedCandleStreamEvent::Item(chart_id, coin.clone(), interval.clone(), None, candle)
        }
        WsStreamEvent::Lagged { skipped } => KeyedCandleStreamEvent::Lagged {
            id: chart_id,
            symbol: coin.clone(),
            interval: interval.clone(),
            hydromancer_key_generation: None,
            skipped,
        },
    }))
}

pub fn ws_spaghetti_candle_stream(
    params: &(
        u64,
        String,
        crate::timeframe::Timeframe,
        Option<crate::spaghetti::Session>,
        Option<crate::timeframe::Timeframe>,
    ),
) -> WsStream<SpaghettiCandleStreamEvent> {
    let spaghetti_id = params.0;
    let coin = params.1.clone();
    let timeframe = params.2;
    let session = params.3;
    let session_granularity = params.4;
    let pair = (params.1.clone(), params.2.api_str().to_string());
    let inner = ws_candle_stream(&pair);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item(candle) => SpaghettiCandleStreamEvent::Item {
            id: spaghetti_id,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: None,
            session,
            session_granularity,
            candle,
        },
        WsStreamEvent::Lagged { skipped } => SpaghettiCandleStreamEvent::Lagged {
            id: spaghetti_id,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: None,
            session,
            session_granularity,
            skipped,
        },
    }))
}
