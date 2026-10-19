//! Per-asset taker fills for the optional Trade Feed context mode.
use super::HydromancerStreamKey;
use super::manager::{
    HydromancerCommand, HydromancerCommandSender, HydromancerRoutedMessage,
    HydromancerSubscriptionGuard, get_hydromancer_trade_feed_manager,
};
use super::parsing::hydromancer_fill_items;
use crate::feed_state::TrackedTradeIntent;
use crate::helpers::parse_finite_number;
use crate::trade_feed_state::{TradeFeedEvent, TradeFeedId, TradePrint};
use crate::wallet_state::address_book::normalize_wallet_address_value;
use crate::ws::WsStream;
use futures::SinkExt;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;

const BATCH_CAPACITY: usize = 512;
const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) fn ws_hydromancer_trade_feed(
    params: &(HydromancerStreamKey, TradeFeedId, u64, String),
) -> WsStream<(TradeFeedId, u64, String, TradeFeedEvent)> {
    let (key, id, generation, coin) = params.clone();
    Box::pin(iced::stream::channel(16, async move |output| {
        forward_trade_feed(
            (id, generation, coin),
            get_hydromancer_trade_feed_manager(key),
            output,
        )
        .await;
    }))
}

fn subscription(coin: &str) -> Option<(String, Value)> {
    // An empty filter must never become the permission-gated all-coins stream.
    if coin.trim().is_empty() {
        return None;
    }
    Some((
        format!("allFills:{coin}"),
        serde_json::json!({
            "type": "subscribe", "subscription": {
                "type": "allFills", "coins": [coin], "aggregateByTime": false
            }
        }),
    ))
}

fn parse_fill(item: &Value, coin: &str) -> Option<TradePrint> {
    let tuple = item.as_array()?;
    let details = tuple.get(1)?;
    // Both counterparties have fills. Show only the initiating trader, matching
    // the native tape and preventing doubled notional/trade counts.
    if details.get("crossed").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let mut trade = crate::ws::market_streams::parse_trade(details, coin)?;
    trade.trader = tuple
        .first()
        .and_then(Value::as_str)
        .filter(|address| address.len() == 42)
        .and_then(normalize_wallet_address_value)
        .map(Into::into);
    // Spot Buy/Sell and unknown directions must not acquire perp position labels.
    if !coin.starts_with(['@', '#'])
        && !coin.contains('/')
        && matches!(
            details.get("dir").and_then(Value::as_str),
            Some(
                "Open Long"
                    | "Open Short"
                    | "Close Long"
                    | "Close Short"
                    | "Long > Short"
                    | "Short > Long"
            )
        )
    {
        let start = details
            .get("startPosition")
            .and_then(Value::as_str)
            .filter(|raw| raw.len() <= 64)
            .and_then(parse_finite_number);
        trade.intent =
            TrackedTradeIntent::from_fill(start, trade.is_buy, parse_finite_number(&trade.size)?);
    }
    Some(trade)
}

fn matches_scope(value: &Value, coin: &str) -> bool {
    let scope = value.get("subscription").unwrap_or(value);
    if scope
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| {
            !matches!(
                kind,
                "allFills" | "error" | "subscriptionUpdate" | "subscriptionResponse"
            )
        })
    {
        return false;
    }
    if let Some(single) = scope.get("coin").and_then(Value::as_str) {
        return single == coin
            || scope
                .get("coins")
                .and_then(Value::as_array)
                .is_some_and(|coins| coins.iter().any(|item| item.as_str() == Some(coin)));
    }
    scope
        .get("coins")
        .and_then(Value::as_array)
        .is_none_or(|coins| coins.iter().any(|item| item.as_str() == Some(coin)))
}

fn control_event(message: &HydromancerRoutedMessage, coin: &str) -> Option<TradeFeedEvent> {
    let value = message.data.as_ref();
    if matches!(message.msg_type.as_str(), "reconnecting" | "disconnected") {
        return Some(TradeFeedEvent::ContextUnavailable);
    }
    if !matches!(
        message.msg_type.as_str(),
        "error" | "subscriptionResponse" | "subscriptionUpdate"
    ) || value.get("operation").and_then(Value::as_str) == Some("unsubscribe")
        || !matches_scope(value, coin)
    {
        return None;
    }
    let failed = value
        .get("failed")
        .and_then(Value::as_array)
        .is_some_and(|failures| {
            failures.iter().any(|failure| {
                failure.as_str() == Some("allFills")
                    || (failure.is_object() && matches_scope(failure, coin))
            })
        });
    if message.msg_type == "error"
        || failed
        || value.get("success").and_then(Value::as_bool) == Some(false)
        || value.get("error").is_some_and(|error| !error.is_null())
    {
        return Some(TradeFeedEvent::ContextUnavailable);
    }
    if value.pointer("/subscription/type").and_then(Value::as_str) == Some("allFills")
        || value
            .get("subscribed")
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("allFills")))
    {
        return Some(TradeFeedEvent::Subscribed);
    }
    None
}

async fn forward_trade_feed(
    (id, generation, coin): (TradeFeedId, u64, String),
    (commands, mut incoming): (
        HydromancerCommandSender,
        broadcast::Receiver<HydromancerRoutedMessage>,
    ),
    mut output: impl futures::Sink<(TradeFeedId, u64, String, TradeFeedEvent)> + Unpin,
) {
    let Some((topic, payload)) = subscription(&coin) else {
        let _ = output
            .send((id, generation, coin, TradeFeedEvent::ContextUnavailable))
            .await;
        return;
    };
    if commands
        .send(HydromancerCommand::Subscribe {
            topic: topic.clone(),
            payload: payload.clone(),
        })
        .is_err()
    {
        let _ = output
            .send((id, generation, coin, TradeFeedEvent::ContextUnavailable))
            .await;
        return;
    }
    let guard = HydromancerSubscriptionGuard::new(commands, vec![(topic, payload)]);
    let mut pending = Vec::new();
    let mut subscribed = false;
    let deadline = tokio::time::sleep(SUBSCRIBE_TIMEOUT);
    tokio::pin!(deadline);
    let mut flush = tokio::time::interval(Duration::from_millis(50));
    flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let event = tokio::select! {
            _ = &mut deadline, if !subscribed => TradeFeedEvent::ContextUnavailable,
            _ = flush.tick() => {
                if pending.is_empty() { continue; }
                TradeFeedEvent::Trades(std::mem::take(&mut pending))
            }
            message = incoming.recv() => match message {
                Ok(message) => {
                    if let Some(event) = control_event(&message, &coin) {
                        match event {
                            TradeFeedEvent::Subscribed if subscribed => continue,
                            TradeFeedEvent::Subscribed => { subscribed = true; event },
                            _ => event,
                        }
                    } else if let Some(items) = hydromancer_fill_items(&message.data, "allFills") {
                        for item in items {
                            let Some(trade) = parse_fill(item, &coin) else { continue; };
                            subscribed = true;
                            pending.push(trade);
                            if pending.len() >= BATCH_CAPACITY
                                && output.send((id, generation, coin.clone(),
                                    TradeFeedEvent::Trades(std::mem::take(&mut pending)))).await.is_err() { return; }
                        }
                        continue;
                    } else { continue; }
                }
                // The app replaces this stream with native Hyperliquid and
                // invalidates the generation. Do not repeatedly consume quota.
                Err(_) => TradeFeedEvent::ContextUnavailable,
            }
        };
        if matches!(event, TradeFeedEvent::ContextUnavailable) {
            drop(guard);
            let _ = output.send((id, generation, coin, event)).await;
            return;
        }
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
