use super::HYDROMANCER_RECONNECT_DELAY_SECS;
use super::HydromancerWsMessage;
use super::manager::{HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_manager};
use super::parsing::hydromancer_control_message;
use crate::account::AssetContext;
use crate::api::{Candle, OrderBook, parse_ws_book};
use crate::ws::WsStream;

use futures::{SinkExt as _, StreamExt as _};
use serde_json::Value;
use tokio::sync::broadcast;

type BookSigfigs = (Option<u8>, Option<u8>);
type KeyedBookStream = WsStream<(u64, String, BookSigfigs, OrderBook)>;
type KeyedAssetContextStream = WsStream<(u64, String, AssetContext)>;
type SymbolAssetContextStream = WsStream<(String, AssetContext)>;
type KeyedCandleStream = WsStream<(u64, String, String, Candle)>;
type SpaghettiCandleStream = WsStream<(u64, String, Candle)>;

// ---------------------------------------------------------------------------
// Hydromancer Market Streams
// ---------------------------------------------------------------------------

pub fn ws_hydromancer_book_stream_keyed(
    params: &(String, u64, String, BookSigfigs),
) -> KeyedBookStream {
    let api_key = params.0.clone();
    let id = params.1;
    let coin = params.2.clone();
    let sigfigs = params.3;

    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(api_key);
        let (topic, payload) = hydromancer_l2_book_subscription(&coin, sigfigs);
        if cmd_tx
            .send(HydromancerCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let guard = HydromancerSubscriptionGuard::new(cmd_tx, vec![topic]);

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if let Some(control) =
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                    {
                        if hydromancer_market_control_should_fallback(&control) {
                            drop(guard);
                            let mut fallback =
                                crate::ws::ws_book_stream_keyed(&(id, coin.clone(), sigfigs));
                            while let Some(item) = fallback.next().await {
                                if output.send(item).await.is_err() {
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
                        if let Some(book) = parse_ws_book(item)
                            && output
                                .send((id, coin.clone(), sigfigs, book))
                                .await
                                .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    ))
                    .await;
                }
            }
        }
    }))
}

pub fn ws_hydromancer_asset_ctx_stream_keyed(
    params: &(String, u64, String),
) -> KeyedAssetContextStream {
    let api_key = params.0.clone();
    let id = params.1;
    let coin = params.2.clone();
    let inner = hydromancer_asset_ctx_stream(api_key, coin.clone());
    Box::pin(futures::StreamExt::map(inner, move |(_coin, ctx)| {
        (id, coin.clone(), ctx)
    }))
}

pub fn ws_hydromancer_asset_ctx_stream_symbol(
    params: &(String, String),
) -> SymbolAssetContextStream {
    hydromancer_asset_ctx_stream(params.0.clone(), params.1.clone())
}

pub fn ws_hydromancer_candle_stream_keyed(
    params: &(String, u64, String, String),
) -> KeyedCandleStream {
    let api_key = params.0.clone();
    let id = params.1;
    let coin = params.2.clone();
    let interval = params.3.clone();
    let inner = hydromancer_candle_stream(api_key, coin.clone(), interval.clone());
    Box::pin(futures::StreamExt::map(inner, move |candle| {
        (id, coin.clone(), interval.clone(), candle)
    }))
}

pub fn ws_hydromancer_spaghetti_candle_stream(
    params: &(String, u64, String, String),
) -> SpaghettiCandleStream {
    let api_key = params.0.clone();
    let id = params.1;
    let coin = params.2.clone();
    let interval = params.3.clone();
    let inner = hydromancer_candle_stream(api_key, coin.clone(), interval);
    Box::pin(futures::StreamExt::map(inner, move |candle| {
        (id, coin.clone(), candle)
    }))
}

fn hydromancer_asset_ctx_stream(api_key: String, coin: String) -> WsStream<(String, AssetContext)> {
    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(api_key);
        let (topic, payload) = hydromancer_asset_ctx_subscription(&coin);
        if cmd_tx
            .send(HydromancerCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let guard = HydromancerSubscriptionGuard::new(cmd_tx, vec![topic]);

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
                            while let Some(item) = fallback.next().await {
                                if output.send(item).await.is_err() {
                                    return;
                                }
                            }
                            return;
                        }
                        continue;
                    }
                    if msg.msg_type != "activeAssetCtx" {
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
                            && output.send((coin.clone(), ctx)).await.is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    ))
                    .await;
                }
            }
        }
    }))
}

fn hydromancer_candle_stream(api_key: String, coin: String, interval: String) -> WsStream<Candle> {
    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(api_key);
        let (topic, payload) = hydromancer_candle_subscription(&coin, &interval);
        if cmd_tx
            .send(HydromancerCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let guard = HydromancerSubscriptionGuard::new(cmd_tx, vec![topic]);

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if let Some(control) =
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                    {
                        if hydromancer_market_control_should_fallback(&control) {
                            drop(guard);
                            let mut fallback = crate::ws::ws_candle_stream_keyed(&(
                                0,
                                coin.clone(),
                                interval.clone(),
                            ));
                            while let Some((_, _, _, candle)) = fallback.next().await {
                                if output.send(candle).await.is_err() {
                                    return;
                                }
                            }
                            return;
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
                        if let Ok(candle) = serde_json::from_value::<Candle>(item.clone())
                            && output.send(candle).await.is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => {
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

fn hydromancer_market_control_should_fallback(control: &HydromancerWsMessage) -> bool {
    matches!(
        control,
        HydromancerWsMessage::Reconnecting { .. } | HydromancerWsMessage::Disconnected(_)
    )
}

fn l2_book_items(value: &Value) -> Vec<&Value> {
    if value.get("coin").is_some() && value.get("levels").is_some() {
        return vec![value];
    }
    if let Some(data) = value.get("data") {
        if data.get("coin").is_some() && data.get("levels").is_some() {
            return vec![data];
        }
        if let Some(items) = data.as_array() {
            return items.iter().collect();
        }
    }
    if let Some(items) = value.get("books").and_then(Value::as_array) {
        return items.iter().collect();
    }
    Vec::new()
}

fn active_asset_ctx_items(value: &Value) -> Vec<&Value> {
    if value.get("coin").is_some() && value.get("ctx").is_some() {
        return vec![value];
    }
    if let Some(data) = value.get("data") {
        if data.get("coin").is_some() && data.get("ctx").is_some() {
            return vec![data];
        }
        if let Some(items) = data.as_array() {
            return items.iter().collect();
        }
    }
    Vec::new()
}

fn candle_items(value: &Value) -> Vec<Value> {
    if value.get("s").is_some() && value.get("i").is_some() {
        return vec![value.clone()];
    }
    if let Some(data) = value.get("data") {
        if data.get("s").is_some() && data.get("i").is_some() {
            return vec![data.clone()];
        }
        if let Some(items) = data.as_array() {
            return items.clone();
        }
    }
    if let Some(candle) = value.get("candle") {
        return vec![candle.clone()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_book_items_accept_single_object_and_data_array_batches() {
        let single = serde_json::json!({
            "channel": "l2Book",
            "data": {
                "coin": "BTC",
                "levels": [
                    [{ "px": "100", "sz": "1" }],
                    [{ "px": "101", "sz": "2" }]
                ]
            }
        });
        let single_items = l2_book_items(&single);
        assert_eq!(single_items.len(), 1);
        assert_eq!(
            single_items[0].get("coin").and_then(Value::as_str),
            Some("BTC")
        );
        assert!(parse_ws_book(single_items[0]).is_some());

        let batch = serde_json::json!({
            "type": "l2Book",
            "data": [
                {
                    "coin": "BTC",
                    "levels": [
                        [{ "px": "100", "sz": "1" }],
                        [{ "px": "101", "sz": "2" }]
                    ]
                },
                {
                    "coin": "ETH",
                    "levels": [
                        [{ "px": "10", "sz": "3" }],
                        [{ "px": "11", "sz": "4" }]
                    ]
                }
            ]
        });
        let batch_items = l2_book_items(&batch);
        assert_eq!(batch_items.len(), 2);
        assert_eq!(
            batch_items[1].get("coin").and_then(Value::as_str),
            Some("ETH")
        );
    }

    #[test]
    fn active_asset_ctx_items_accept_data_objects() {
        let value = serde_json::json!({
            "channel": "activeAssetCtx",
            "data": {
                "coin": "ETH",
                "ctx": {
                    "oraclePx": "3230.1",
                    "markPx": "3227.4",
                    "midPx": "3228.25"
                }
            }
        });

        let items = active_asset_ctx_items(&value);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("coin").and_then(Value::as_str), Some("ETH"));
        let ctx = items[0].get("ctx").expect("ctx exists").clone();
        let ctx = serde_json::from_value::<AssetContext>(ctx).expect("ctx parses");
        assert_eq!(ctx.mid_px.as_deref(), Some("3228.25"));
    }

    #[test]
    fn candle_items_accept_data_array_batches() {
        let value = serde_json::json!({
            "channel": "candle",
            "data": [
                {
                    "s": "BTC",
                    "i": "1m",
                    "t": 10,
                    "T": 69,
                    "o": "1",
                    "h": "2",
                    "l": "0.5",
                    "c": "1.5",
                    "v": "12"
                }
            ]
        });

        let items = candle_items(&value);
        assert_eq!(items.len(), 1);
        let candle = serde_json::from_value::<Candle>(items[0].clone()).expect("candle parses");
        assert_eq!(candle.open_time, 10);
        assert_eq!(candle.close, 1.5);
    }
}
