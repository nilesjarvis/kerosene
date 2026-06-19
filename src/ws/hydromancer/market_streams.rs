use super::manager::{HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_manager};
use super::parsing::hydromancer_control_message;
use super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerStreamKey, HydromancerWsMessage};
use crate::account::AssetContext;
use crate::api::{Candle, parse_ws_book};
use crate::ws::{
    KeyedAssetContextStreamEvent, KeyedBookStreamEvent, KeyedCandleStreamEvent,
    SpaghettiCandleStreamEvent, SymbolAssetContextStreamEvent, WsStream, WsStreamEvent,
    l2_book_payload_matches_sigfigs,
};

use futures::{SinkExt as _, StreamExt as _};
use serde_json::Value;
use tokio::sync::broadcast;

type BookSigfigs = (Option<u8>, Option<u8>);
type KeyedBookEventStream = WsStream<KeyedBookStreamEvent>;
type KeyedAssetContextStream = WsStream<KeyedAssetContextStreamEvent>;
type SymbolAssetContextStream = WsStream<SymbolAssetContextStreamEvent>;
type KeyedCandleStream = WsStream<KeyedCandleStreamEvent>;
type SpaghettiCandleStream = WsStream<SpaghettiCandleStreamEvent>;

// ---------------------------------------------------------------------------
// Hydromancer Market Streams
// ---------------------------------------------------------------------------

pub fn ws_hydromancer_book_stream_keyed_events(
    params: &(HydromancerStreamKey, u64, String, BookSigfigs),
) -> KeyedBookEventStream {
    let stream_key = params.0.clone();
    let hydromancer_key_generation = params.0.generation();
    let id = params.1;
    let coin = params.2.clone();
    let sigfigs = params.3;

    Box::pin(iced::stream::channel(10, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(stream_key);
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
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                    {
                        if hydromancer_market_control_should_fallback(&control) {
                            drop(guard);
                            let mut fallback = crate::ws::ws_book_stream_keyed_events(&(
                                id,
                                coin.clone(),
                                sigfigs,
                            ));
                            while let Some(event) = fallback.next().await {
                                let scoped_event = match event {
                                    KeyedBookStreamEvent::Item(id, coin, sigfigs, _, book) => {
                                        KeyedBookStreamEvent::Item(
                                            id,
                                            coin,
                                            sigfigs,
                                            Some(hydromancer_key_generation),
                                            book,
                                        )
                                    }
                                    KeyedBookStreamEvent::Lagged {
                                        id,
                                        coin,
                                        sigfigs,
                                        skipped,
                                        ..
                                    } => KeyedBookStreamEvent::Lagged {
                                        id,
                                        coin,
                                        sigfigs,
                                        hydromancer_key_generation: Some(
                                            hydromancer_key_generation,
                                        ),
                                        skipped,
                                    },
                                };
                                if output.send(scoped_event).await.is_err() {
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
                    if !super::emit_hydromancer_lag_after_reconnect(
                        &reconnect_tx,
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

pub fn ws_hydromancer_asset_ctx_stream_keyed(
    params: &(HydromancerStreamKey, u64, String),
) -> KeyedAssetContextStream {
    let stream_key = params.0.clone();
    let hydromancer_key_generation = params.0.generation();
    let id = params.1;
    let coin = params.2.clone();
    let inner = hydromancer_asset_ctx_stream(stream_key, coin.clone());
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item((_coin, ctx)) => KeyedAssetContextStreamEvent::Item(
            id,
            coin.clone(),
            Some(hydromancer_key_generation),
            Box::new(ctx),
        ),
        WsStreamEvent::Lagged { skipped } => KeyedAssetContextStreamEvent::Lagged {
            id,
            symbol: coin.clone(),
            hydromancer_key_generation: Some(hydromancer_key_generation),
            skipped,
        },
    }))
}

pub fn ws_hydromancer_asset_ctx_stream_symbol(
    params: &(HydromancerStreamKey, String),
) -> SymbolAssetContextStream {
    let hydromancer_key_generation = params.0.generation();
    let symbol = params.1.clone();
    let inner = hydromancer_asset_ctx_stream(params.0.clone(), params.1.clone());
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item((symbol, ctx)) => SymbolAssetContextStreamEvent::Item(
            symbol,
            Some(hydromancer_key_generation),
            Box::new(ctx),
        ),
        WsStreamEvent::Lagged { skipped } => SymbolAssetContextStreamEvent::Lagged {
            symbol: symbol.clone(),
            hydromancer_key_generation: Some(hydromancer_key_generation),
            skipped,
        },
    }))
}

pub fn ws_hydromancer_candle_stream_keyed(
    params: &(HydromancerStreamKey, u64, String, String),
) -> KeyedCandleStream {
    let stream_key = params.0.clone();
    let hydromancer_key_generation = params.0.generation();
    let id = params.1;
    let coin = params.2.clone();
    let interval = params.3.clone();
    let inner = hydromancer_candle_stream(stream_key, coin.clone(), interval.clone());
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item(candle) => KeyedCandleStreamEvent::Item(
            id,
            coin.clone(),
            interval.clone(),
            Some(hydromancer_key_generation),
            candle,
        ),
        WsStreamEvent::Lagged { skipped } => KeyedCandleStreamEvent::Lagged {
            id,
            symbol: coin.clone(),
            interval: interval.clone(),
            hydromancer_key_generation: Some(hydromancer_key_generation),
            skipped,
        },
    }))
}

pub fn ws_hydromancer_spaghetti_candle_stream(
    params: &(
        HydromancerStreamKey,
        u64,
        String,
        crate::timeframe::Timeframe,
        Option<crate::spaghetti::Session>,
        Option<crate::timeframe::Timeframe>,
    ),
) -> SpaghettiCandleStream {
    let stream_key = params.0.clone();
    let hydromancer_key_generation = params.0.generation();
    let id = params.1;
    let coin = params.2.clone();
    let timeframe = params.3;
    let session = params.4;
    let session_granularity = params.5;
    let interval = params.3.api_str().to_string();
    let inner = hydromancer_candle_stream(stream_key, coin.clone(), interval);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        WsStreamEvent::Item(candle) => SpaghettiCandleStreamEvent::Item {
            id,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: Some(hydromancer_key_generation),
            session,
            session_granularity,
            candle,
        },
        WsStreamEvent::Lagged { skipped } => SpaghettiCandleStreamEvent::Lagged {
            id,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: Some(hydromancer_key_generation),
            session,
            session_granularity,
            skipped,
        },
    }))
}

fn hydromancer_asset_ctx_stream(
    stream_key: HydromancerStreamKey,
    coin: String,
) -> WsStream<WsStreamEvent<(String, AssetContext)>> {
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
                                    SymbolAssetContextStreamEvent::Item(symbol, _, ctx) => {
                                        WsStreamEvent::Item((symbol, *ctx))
                                    }
                                    SymbolAssetContextStreamEvent::Lagged { skipped, .. } => {
                                        WsStreamEvent::Lagged { skipped }
                                    }
                                };
                                if output.send(event).await.is_err() {
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
                            && output
                                .send(WsStreamEvent::Item((coin.clone(), ctx)))
                                .await
                                .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    if !super::emit_hydromancer_lag_after_reconnect(
                        &reconnect_tx,
                        WsStreamEvent::Lagged { skipped },
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

fn hydromancer_candle_stream(
    stream_key: HydromancerStreamKey,
    coin: String,
    interval: String,
) -> WsStream<WsStreamEvent<Candle>> {
    Box::pin(iced::stream::channel(10, async move |mut output| {
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
                            while let Some(event) = fallback.next().await {
                                match event {
                                    KeyedCandleStreamEvent::Item(_, _, _, _, candle) => {
                                        if output.send(WsStreamEvent::Item(candle)).await.is_err() {
                                            return;
                                        }
                                    }
                                    KeyedCandleStreamEvent::Lagged { skipped, .. } => {
                                        if output
                                            .send(WsStreamEvent::Lagged { skipped })
                                            .await
                                            .is_err()
                                        {
                                            return;
                                        }
                                    }
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
                            && output.send(WsStreamEvent::Item(candle)).await.is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    if !super::emit_hydromancer_lag_after_reconnect(
                        &reconnect_tx,
                        WsStreamEvent::Lagged { skipped },
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
    match control {
        HydromancerWsMessage::Reconnecting { error, .. } => {
            hydromancer_market_disconnect_should_fallback(error)
        }
        HydromancerWsMessage::Disconnected(error) => {
            hydromancer_market_disconnect_should_fallback(error)
        }
        _ => false,
    }
}

fn hydromancer_market_disconnect_should_fallback(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("authentication failed")
        || lower.contains("check the api key")
        || lower.contains("unauthorized")
        || lower.contains("unauthenticated")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid token")
        || lower.contains("http 401")
        || lower.contains("http 403")
        || ((lower.contains("subscription")
            || lower.contains("subscribe")
            || lower.contains("quota")
            || lower.contains("too many"))
            && (lower.contains("rejected")
                || lower.contains("denied")
                || lower.contains("unsupported")
                || lower.contains("not supported")
                || lower.contains("too many")
                || lower.contains("quota")))
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

    #[test]
    fn market_stream_fallback_ignores_transient_reconnect_controls() {
        assert!(!hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Connecting
        ));
        assert!(!hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Reconnected
        ));
        assert!(!hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Reconnecting {
                error: "network timeout".to_string(),
                retry_delay_secs: 2,
            }
        ));
        assert!(hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Reconnecting {
                error:
                    "Hydromancer authentication failed. Check the API key in Settings > Integrations."
                        .to_string(),
                retry_delay_secs: 2,
            }
        ));
        assert!(hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Reconnecting {
                error: "subscription rejected: too many subscriptions".to_string(),
                retry_delay_secs: 2,
            }
        ));
        assert!(!hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Disconnected("stream disconnected".to_string())
        ));
        assert!(!hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Disconnected(
                "Hydromancer network timeout: heartbeat timeout after 95s".to_string()
            )
        ));
        assert!(hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Disconnected(
                "Hydromancer authentication failed. Check the API key in Settings > Integrations."
                    .to_string()
            )
        ));
        assert!(hydromancer_market_control_should_fallback(
            &HydromancerWsMessage::Disconnected(
                "subscription rejected: too many subscriptions".to_string()
            )
        ));
    }
}
