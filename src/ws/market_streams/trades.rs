use crate::helpers::parse_finite_number;
use crate::trade_feed_state::{TradeFeedEvent, TradeFeedId, TradePrint};
use crate::ws::manager::WsRoutedMessage;
use crate::ws::{SubscriptionGuard, WsCommand, WsCommandSender, WsStream, get_manager};
use futures::SinkExt;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;

const BATCH_CAPACITY: usize = 512;

pub(crate) fn parse_trade(value: &Value, coin: &str) -> Option<TradePrint> {
    if value.get("coin")?.as_str()? != coin {
        return None;
    }
    let is_buy = match value.get("side")?.as_str()? {
        "B" => true,
        "A" => false,
        _ => return None,
    };
    let price = value.get("px")?.as_str()?;
    let size = value.get("sz")?.as_str()?;
    // Bound strings copied into UI state, including malformed public payloads.
    if price.len() > 64 || size.len() > 64 {
        return None;
    }
    let px = parse_finite_number(price)?;
    let sz = parse_finite_number(size)?;
    let notional = px * sz;
    let time_ms = value.get("time")?.as_u64()?;
    if px <= 0.0
        || sz <= 0.0
        || !notional.is_finite()
        || notional <= 0.0
        || time_ms == 0
        || time_ms > i64::MAX as u64
    {
        return None;
    }
    Some(TradePrint {
        time_ms,
        tid: value.get("tid")?.as_u64()?,
        is_buy,
        price: price.to_owned(),
        size: size.to_owned(),
        value: notional,
    })
}

pub(crate) fn ws_trade_feed(
    params: &(TradeFeedId, u64, String),
) -> WsStream<(TradeFeedId, u64, String, TradeFeedEvent)> {
    let params = params.clone();
    Box::pin(iced::stream::channel(16, async move |output| {
        forward_trade_feed(params, get_manager(), output).await;
    }))
}

async fn forward_trade_feed(
    (id, generation, coin): (TradeFeedId, u64, String),
    (cmd_tx, mut rx): (WsCommandSender, broadcast::Receiver<WsRoutedMessage>),
    mut output: impl futures::Sink<(TradeFeedId, u64, String, TradeFeedEvent)> + Unpin,
) {
    let topic = format!("trades:{coin}");
    let payload = serde_json::json!({
        "method": "subscribe", "subscription": {"type": "trades", "coin": coin}
    });
    if cmd_tx
        .send(WsCommand::Subscribe {
            topic: topic.clone(),
            payload: payload.clone(),
        })
        .is_err()
    {
        let _ = output
            .send((id, generation, coin, TradeFeedEvent::Unavailable))
            .await;
        return;
    }
    let _guard = SubscriptionGuard {
        cmd_tx: cmd_tx.clone(),
        subscriptions: vec![(topic, payload)],
    };
    let mut pending = Vec::new();
    let mut flush = tokio::time::interval(Duration::from_millis(50));
    flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let event = tokio::select! {
            _ = flush.tick() => {
                if pending.is_empty() { continue; }
                TradeFeedEvent::Trades(std::mem::take(&mut pending))
            }
            received = rx.recv() => match received {
                Ok(message) if message.channel == "trades" => {
                    if let Some(items) = message.data.as_array() {
                        for trade in items.iter().filter_map(|item| parse_trade(item, &coin)) {
                            pending.push(trade);
                            if pending.len() >= BATCH_CAPACITY {
                                let event = TradeFeedEvent::Trades(std::mem::take(&mut pending));
                                if output.send((id, generation, coin.clone(), event)).await.is_err() { return; }
                            }
                        }
                    }
                    continue;
                }
                Ok(message) if message.channel == "subscriptionResponse"
                    && message.data.pointer("/subscription/type").and_then(Value::as_str) == Some("trades")
                    && message.data.pointer("/subscription/coin").and_then(Value::as_str) == Some(&coin)
                    && message.data.get("method").and_then(Value::as_str) == Some("subscribe") => {
                    TradeFeedEvent::Subscribed
                }
                Ok(message) if message.channel == "keroseneDisconnected" => {
                    // Flush already received prints before showing a discontinuity.
                    if !pending.is_empty() {
                        let event = TradeFeedEvent::Trades(std::mem::take(&mut pending));
                        if output.send((id, generation, coin.clone(), event)).await.is_err() { return; }
                    }
                    TradeFeedEvent::Reconnecting
                }
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    cmd_tx.request_lag_reconnect();
                    TradeFeedEvent::Lagged
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let _ = output.send((id, generation, coin, TradeFeedEvent::Unavailable)).await;
                    return;
                }
            }
        };
        if output
            .send((id, generation, coin.clone(), event))
            .await
            .is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
