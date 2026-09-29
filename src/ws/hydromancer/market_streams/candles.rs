use super::super::manager::{
    HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_manager,
};
use super::super::parsing::hydromancer_control_message;
use super::super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerStreamKey, HydromancerWsMessage};
use super::hydromancer_market_control_should_fallback;
use super::payloads::candle_items;
use crate::api::Candle;
use crate::ws::{KeyedCandleStreamEvent, SpaghettiCandleStreamEvent, WsStream};
use futures::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
enum HydromancerCandleStreamEvent {
    Unavailable {
        hydromancer_key_generation: Option<u64>,
        reason: String,
    },
    Item(Option<u64>, Candle),
    Lagged {
        hydromancer_key_generation: Option<u64>,
        skipped: u64,
    },
}

pub fn ws_hydromancer_candle_stream_keyed(
    params: &(HydromancerStreamKey, u64, String, String),
) -> WsStream<KeyedCandleStreamEvent> {
    let stream_key = params.0.clone();
    let id = params.1;
    let coin = params.2.clone();
    let interval = params.3.clone();
    let inner = hydromancer_candle_stream(stream_key, coin.clone(), interval.clone());
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        HydromancerCandleStreamEvent::Item(hydromancer_key_generation, candle) => {
            KeyedCandleStreamEvent::Item(
                id,
                coin.clone(),
                interval.clone(),
                hydromancer_key_generation,
                candle,
            )
        }
        HydromancerCandleStreamEvent::Unavailable {
            hydromancer_key_generation,
            reason,
        } => KeyedCandleStreamEvent::Unavailable {
            id,
            symbol: coin.clone(),
            interval: interval.clone(),
            hydromancer_key_generation,
            reason,
        },
        HydromancerCandleStreamEvent::Lagged {
            hydromancer_key_generation,
            skipped,
        } => KeyedCandleStreamEvent::Lagged {
            id,
            symbol: coin.clone(),
            interval: interval.clone(),
            hydromancer_key_generation,
            skipped,
        },
    }))
}

pub fn ws_hydromancer_spaghetti_candle_stream(
    params: &(
        HydromancerStreamKey,
        u64,
        u64,
        String,
        crate::timeframe::Timeframe,
        Option<crate::spaghetti::Session>,
        Option<crate::timeframe::Timeframe>,
    ),
) -> WsStream<SpaghettiCandleStreamEvent> {
    let stream_key = params.0.clone();
    let id = params.1;
    let instance_epoch = params.2;
    let coin = params.3.clone();
    let timeframe = params.4;
    let session = params.5;
    let session_granularity = params.6;
    let interval = params.4.api_str().to_string();
    let inner = hydromancer_candle_stream(stream_key, coin.clone(), interval);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        HydromancerCandleStreamEvent::Item(hydromancer_key_generation, candle) => {
            SpaghettiCandleStreamEvent::Item {
                id,
                instance_epoch,
                symbol: coin.clone(),
                timeframe,
                hydromancer_key_generation,
                session,
                session_granularity,
                candle,
            }
        }
        HydromancerCandleStreamEvent::Unavailable {
            hydromancer_key_generation,
            reason,
        } => SpaghettiCandleStreamEvent::Unavailable {
            id,
            instance_epoch,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation,
            session,
            session_granularity,
            reason,
        },
        HydromancerCandleStreamEvent::Lagged {
            hydromancer_key_generation,
            skipped,
        } => SpaghettiCandleStreamEvent::Lagged {
            id,
            instance_epoch,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation,
            session,
            session_granularity,
            skipped,
        },
    }))
}

fn hydromancer_candle_stream(
    stream_key: HydromancerStreamKey,
    coin: String,
    interval: String,
) -> WsStream<HydromancerCandleStreamEvent> {
    let hydromancer_key_generation = stream_key.generation();
    Box::pin(iced::stream::channel(128, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(stream_key);
        let (topic, payload) = hydromancer_candle_subscription(&coin, &interval);
        let subscription = (topic.clone(), payload.clone());
        if cmd_tx
            .send(HydromancerCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let reconnect_tx = cmd_tx.clone();
        let guard = HydromancerSubscriptionGuard::new(cmd_tx, vec![subscription]);

        let mut watchdog = crate::ws::market_streams::CandleWatchdog::new(&coin, &interval);
        loop {
            let Some(message) = watchdog.recv(&mut msg_rx).await else {
                let _ = reconnect_tx.send(HydromancerCommand::Resubscribe {
                    topic: topic.clone(),
                });
                if output
                    .send(HydromancerCandleStreamEvent::Unavailable {
                        hydromancer_key_generation: Some(hydromancer_key_generation),
                        reason: "Candle stream quiet · verifying history".to_string(),
                    })
                    .await
                    .is_err()
                {
                    return;
                }
                continue;
            };
            match message {
                Ok(msg) => {
                    if let Some(control) =
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                    {
                        if interval != "1s" && hydromancer_market_control_should_fallback(&control)
                        {
                            drop(guard);
                            let mut fallback = crate::ws::ws_candle_stream_keyed(&(
                                0,
                                coin.clone(),
                                interval.clone(),
                            ));
                            while let Some(event) = fallback.next().await {
                                let event = match event {
                                    KeyedCandleStreamEvent::Item(
                                        _,
                                        _,
                                        _,
                                        hydromancer_key_generation,
                                        candle,
                                    ) => HydromancerCandleStreamEvent::Item(
                                        hydromancer_key_generation,
                                        candle,
                                    ),
                                    KeyedCandleStreamEvent::Unavailable {
                                        hydromancer_key_generation,
                                        reason,
                                        ..
                                    } => HydromancerCandleStreamEvent::Unavailable {
                                        hydromancer_key_generation,
                                        reason,
                                    },
                                    KeyedCandleStreamEvent::Lagged {
                                        hydromancer_key_generation,
                                        skipped,
                                        ..
                                    } => HydromancerCandleStreamEvent::Lagged {
                                        hydromancer_key_generation,
                                        skipped,
                                    },
                                };
                                if output.send(event).await.is_err() {
                                    return;
                                }
                            }
                            return;
                        }
                        if matches!(control, HydromancerWsMessage::Disconnected(_))
                            && watchdog.report_error_due()
                        {
                            // Provider errors may lack a topic. Surface a bounded,
                            // redacted status; periodic probes retry only quiet topics.
                            if output.send(HydromancerCandleStreamEvent::Unavailable { hydromancer_key_generation: Some(hydromancer_key_generation), reason: "Candle subscription unavailable · check provider limits or credentials".to_string() }).await.is_err() { return; }
                        }
                        continue;
                    }
                    if msg.msg_type != "candle" {
                        continue;
                    }
                    for item in candle_items(msg.data.as_ref()) {
                        if item.get("s").and_then(Value::as_str) != Some(coin.as_str())
                            || item.get("i").and_then(Value::as_str) != Some(interval.as_str())
                        {
                            continue;
                        }
                        if let Ok(candle) = Candle::deserialize(item)
                            && crate::api::is_valid_candle(&candle)
                        {
                            watchdog.mark_valid();
                            if output
                                .send(HydromancerCandleStreamEvent::Item(
                                    Some(hydromancer_key_generation),
                                    candle,
                                ))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    if output
                        .send(HydromancerCandleStreamEvent::Lagged {
                            hydromancer_key_generation: Some(hydromancer_key_generation),
                            skipped,
                        })
                        .await
                        .is_err()
                    {
                        return;
                    }
                }

                Err(error) if crate::ws::broadcast_receiver_closed(&error) => {
                    return;
                }
                Err(_error) => {
                    tokio::time::sleep(std::time::Duration::from_secs(
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    ))
                    .await;
                }
            }
        }
    }))
}

fn hydromancer_candle_subscription(coin: &str, interval: &str) -> (String, Value) {
    (
        format!("candle:{coin}:{interval}"),
        serde_json::json!({
            "method": "subscribe",
            "subscription": {
                "type": "candle",
                "coin": coin,
                "interval": interval,
            }
        }),
    )
}
