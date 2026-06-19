use crate::account::AssetContext;
use crate::ws::{SubscriptionGuard, WsCommand, WsStream, get_manager};

use super::{KeyedAssetContextStreamEvent, SymbolAssetContextStreamEvent, WsStreamEvent};
use futures::SinkExt as _;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Active Asset Context Streams
// ---------------------------------------------------------------------------

fn ws_asset_ctx_stream(
    coin: &str,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = WsStreamEvent<AssetContext>> + Send>> {
    let coin = coin.to_string();

    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_manager();

        let topic = format!("activeAssetCtx:{}", coin);
        let payload = serde_json::json!({
            "method": "subscribe",
            "subscription": {
                "type": "activeAssetCtx",
                "coin": coin,
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
                    if msg.channel == "activeAssetCtx"
                        && msg.data.get("coin").and_then(|v| v.as_str()) == Some(&coin)
                        && let Some(ctx_val) = msg.data.get("ctx")
                        && let Ok(ctx) = serde_json::from_value::<AssetContext>(ctx_val.clone())
                        && output.send(WsStreamEvent::Item(ctx)).await.is_err()
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

pub fn ws_asset_ctx_stream_keyed(params: &(u64, String)) -> WsStream<KeyedAssetContextStreamEvent> {
    let chart_id = params.0;
    let coin = params.1.clone();
    let inner = ws_asset_ctx_stream(&params.1);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item(ctx) => {
            KeyedAssetContextStreamEvent::Item(chart_id, coin.clone(), None, Box::new(ctx))
        }
        WsStreamEvent::Lagged { skipped } => KeyedAssetContextStreamEvent::Lagged {
            id: chart_id,
            symbol: coin.clone(),
            hydromancer_key_generation: None,
            skipped,
        },
    }))
}

pub fn ws_asset_ctx_stream_symbol(params: &(String,)) -> WsStream<SymbolAssetContextStreamEvent> {
    let coin = params.0.clone();
    let inner = ws_asset_ctx_stream(&params.0);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item(ctx) => {
            SymbolAssetContextStreamEvent::Item(coin.clone(), None, Box::new(ctx))
        }
        WsStreamEvent::Lagged { skipped } => SymbolAssetContextStreamEvent::Lagged {
            symbol: coin.clone(),
            hydromancer_key_generation: None,
            skipped,
        },
    }))
}
