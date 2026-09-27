use super::super::manager::{
    HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_manager,
};
use super::super::parsing::hydromancer_control_message;
use super::super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerStreamKey, emit_after_reconnect};
use super::hydromancer_market_control_should_fallback;
use super::payloads::active_asset_ctx_items;
use crate::account::AssetContext;
use crate::ws::market_streams::is_active_asset_ctx_channel;
use crate::ws::{KeyedAssetContextStreamEvent, SymbolAssetContextStreamEvent, WsStream};
use futures::{SinkExt as _, StreamExt as _};
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
enum HydromancerAssetCtxStreamEvent {
    Item(String, Option<u64>, Box<AssetContext>),
    Lagged {
        hydromancer_key_generation: Option<u64>,
        skipped: u64,
    },
}

pub fn ws_hydromancer_asset_ctx_stream_keyed(
    params: &(HydromancerStreamKey, u64, String),
) -> WsStream<KeyedAssetContextStreamEvent> {
    let stream_key = params.0.clone();
    let id = params.1;
    let coin = params.2.clone();
    let inner = hydromancer_asset_ctx_stream(stream_key, coin.clone());
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        HydromancerAssetCtxStreamEvent::Item(_symbol, hydromancer_key_generation, ctx) => {
            KeyedAssetContextStreamEvent::Item(id, coin.clone(), hydromancer_key_generation, ctx)
        }
        HydromancerAssetCtxStreamEvent::Lagged {
            hydromancer_key_generation,
            skipped,
        } => KeyedAssetContextStreamEvent::Lagged {
            id,
            symbol: coin.clone(),
            hydromancer_key_generation,
            skipped,
        },
    }))
}

pub fn ws_hydromancer_asset_ctx_stream_symbol(
    params: &(HydromancerStreamKey, String),
) -> WsStream<SymbolAssetContextStreamEvent> {
    let symbol = params.1.clone();
    let inner = hydromancer_asset_ctx_stream(params.0.clone(), params.1.clone());
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        HydromancerAssetCtxStreamEvent::Item(symbol, hydromancer_key_generation, ctx) => {
            SymbolAssetContextStreamEvent::Item(symbol, hydromancer_key_generation, ctx)
        }
        HydromancerAssetCtxStreamEvent::Lagged {
            hydromancer_key_generation,
            skipped,
        } => SymbolAssetContextStreamEvent::Lagged {
            symbol: symbol.clone(),
            hydromancer_key_generation,
            skipped,
        },
    }))
}

fn hydromancer_asset_ctx_stream(
    stream_key: HydromancerStreamKey,
    coin: String,
) -> WsStream<HydromancerAssetCtxStreamEvent> {
    let hydromancer_key_generation = stream_key.generation();
    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(stream_key);
        let (topic, payload) = hydromancer_asset_ctx_subscription(&coin);
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
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                    {
                        if hydromancer_market_control_should_fallback(&control) {
                            drop(guard);
                            let mut fallback =
                                crate::ws::ws_asset_ctx_stream_symbol(&(coin.clone(),));
                            while let Some(event) = fallback.next().await {
                                let event = match event {
                                    SymbolAssetContextStreamEvent::Item(
                                        symbol,
                                        hydromancer_key_generation,
                                        ctx,
                                    ) => HydromancerAssetCtxStreamEvent::Item(
                                        symbol,
                                        hydromancer_key_generation,
                                        ctx,
                                    ),
                                    SymbolAssetContextStreamEvent::Lagged {
                                        hydromancer_key_generation,
                                        skipped,
                                        ..
                                    } => HydromancerAssetCtxStreamEvent::Lagged {
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
                        continue;
                    }
                    // Spot pairs answer an activeAssetCtx subscription on the
                    // activeSpotAssetCtx channel; accept both.
                    if !is_active_asset_ctx_channel(&msg.msg_type) {
                        continue;
                    }
                    for item in active_asset_ctx_items(msg.data.as_ref()) {
                        if item.get("coin").and_then(Value::as_str) != Some(coin.as_str()) {
                            continue;
                        }
                        let Some(ctx_val) = item.get("ctx") else {
                            continue;
                        };
                        if let Ok(ctx) = serde_json::from_value::<AssetContext>(ctx_val.clone())
                            && output
                                .send(HydromancerAssetCtxStreamEvent::Item(
                                    coin.clone(),
                                    Some(hydromancer_key_generation),
                                    Box::new(ctx),
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
                        HydromancerAssetCtxStreamEvent::Lagged {
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

fn hydromancer_asset_ctx_subscription(coin: &str) -> (String, Value) {
    (
        format!("activeAssetCtx:{coin}"),
        serde_json::json!({
            "method": "subscribe",
            "subscription": {
                "type": "activeAssetCtx",
                "coin": coin,
            }
        }),
    )
}
