use crate::account::AssetContext;
use crate::ws::{SubscriptionGuard, WsCommand, WsStream, get_manager};

use super::{KeyedAssetContext, SymbolAssetContext};
use futures::SinkExt as _;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Active Asset Context Streams
// ---------------------------------------------------------------------------

fn ws_asset_ctx_stream(
    coin: &str,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = AssetContext> + Send>> {
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
                    if msg.channel == "activeAssetCtx"
                        && msg.data.get("coin").and_then(|v| v.as_str()) == Some(&coin)
                        && let Some(ctx_val) = msg.data.get("ctx")
                        && let Ok(ctx) = serde_json::from_value::<AssetContext>(ctx_val.clone())
                        && output.send(ctx).await.is_err()
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

pub fn ws_asset_ctx_stream_keyed(params: &(u64, String)) -> WsStream<KeyedAssetContext> {
    let chart_id = params.0;
    let coin = params.1.clone();
    let inner = ws_asset_ctx_stream(&params.1);
    Box::pin(futures::StreamExt::map(inner, move |ctx| {
        (chart_id, coin.clone(), ctx)
    }))
}

pub fn ws_asset_ctx_stream_symbol(params: &(String,)) -> WsStream<SymbolAssetContext> {
    let coin = params.0.clone();
    let inner = ws_asset_ctx_stream(&params.0);
    Box::pin(futures::StreamExt::map(inner, move |ctx| {
        (coin.clone(), ctx)
    }))
}
