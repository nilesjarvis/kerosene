use super::super::manager::{
    HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_book_manager,
};
use super::super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerStreamKey, emit_after_reconnect};
use super::hydromancer_market_control_message;
use super::hydromancer_market_control_should_fallback;
use super::payloads::l2_book_items;
use crate::api::parse_ws_book;
use crate::ws::{KeyedBookStreamEvent, L2BookSigfigs, WsStream, l2_book_payload_matches_sigfigs};
use futures::{SinkExt as _, StreamExt as _};
use serde_json::Value;
use tokio::sync::broadcast;

type BookSigfigs = L2BookSigfigs;

pub fn ws_hydromancer_book_stream_keyed_events(
    params: &(HydromancerStreamKey, u64, String, BookSigfigs),
) -> WsStream<KeyedBookStreamEvent> {
    let stream_key = params.0.clone();
    let hydromancer_key_generation = params.0.generation();
    let id = params.1;
    let coin = params.2.clone();
    let sigfigs = params.3;

    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_book_manager(stream_key, sigfigs);
        let (topic, payload) = hydromancer_l2_book_subscription(&coin, sigfigs);
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

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if let Some(control) =
                        hydromancer_market_control_message(&msg.msg_type, msg.data.as_ref())
                    {
                        if hydromancer_market_control_should_fallback(&control) {
                            drop(guard);
                            let mut fallback = crate::ws::ws_book_stream_keyed_events(&(
                                id,
                                coin.clone(),
                                sigfigs,
                            ));
                            while let Some(event) = fallback.next().await {
                                if output.send(event).await.is_err() {
                                    return;
                                }
                            }
                            return;
                        }
                        continue;
                    }
                    if msg.msg_type != "l2Book" {
                        continue;
                    }
                    for item in l2_book_items(msg.data.as_ref()) {
                        if item.get("coin").and_then(Value::as_str) != Some(coin.as_str()) {
                            continue;
                        }
                        if !l2_book_payload_matches_sigfigs(item, sigfigs) {
                            continue;
                        }
                        if let Some(book) = parse_ws_book(item)
                            && output
                                .send(KeyedBookStreamEvent::Item(
                                    id,
                                    coin.clone(),
                                    sigfigs,
                                    Some(hydromancer_key_generation),
                                    book,
                                ))
                                .await
                                .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    if !emit_after_reconnect(
                        || reconnect_tx.request_lag_reconnect(),
                        KeyedBookStreamEvent::Lagged {
                            id,
                            coin: coin.clone(),
                            sigfigs,
                            hydromancer_key_generation: Some(hydromancer_key_generation),
                            skipped,
                        },
                        |event| async { output.send(event).await.is_ok() },
                        std::time::Duration::from_secs(HYDROMANCER_RECONNECT_DELAY_SECS),
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
                    tokio::time::sleep(std::time::Duration::from_secs(
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    ))
                    .await;
                }
            }
        }
    }))
}

fn hydromancer_l2_book_subscription(coin: &str, sigfigs: BookSigfigs) -> (String, Value) {
    let mut subscription = serde_json::json!({
        "type": "l2Book",
        "coins": [coin],
        "nLevels": 20,
    });
    if let Some(object) = subscription.as_object_mut() {
        if let Some(n) = sigfigs.0 {
            object.insert("nSigFigs".to_string(), serde_json::json!(n));
        }
        if let Some(m) = sigfigs.1 {
            object.insert("mantissa".to_string(), serde_json::json!(m));
        }
    }

    (
        format!(
            "l2Book:{}:{}:{}",
            coin,
            sigfigs.0.unwrap_or(0),
            sigfigs.1.unwrap_or(0)
        ),
        serde_json::json!({
            "method": "subscribe",
            "subscription": subscription,
        }),
    )
}
