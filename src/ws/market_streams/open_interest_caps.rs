use crate::chart_state::{
    OPEN_INTEREST_CAP_STREAM_MAX_AGE_MS, OpenInterestCapSnapshot, OpenInterestCapStreamEvent,
};
use crate::ws::manager::WsRoutedMessage;
use crate::ws::{SubscriptionGuard, WsCommand, WsCommandSender, WsStream, get_manager};
use futures::SinkExt;
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::Instant;

// Cap lists are public and identical for all users. A fixed null address keeps
// this subscription independent of wallets and avoids receiving real account data.
const CAP_STATUS_USER: &str = "0x0000000000000000000000000000000000000000";
const CAP_STATUS_TOPIC: &str = "webData3:open-interest-caps";
const STALE_AFTER: Duration = Duration::from_millis(OPEN_INTEREST_CAP_STREAM_MAX_AGE_MS);

fn parse_snapshot(data: &Value, received_at_ms: u64) -> Result<OpenInterestCapSnapshot, ()> {
    let server_time_ms = data
        .pointer("/userState/serverTime")
        .and_then(Value::as_u64)
        .filter(|time| *time > 0)
        .ok_or(())?;
    let states = data
        .get("perpDexStates")
        .and_then(Value::as_array)
        .filter(|states| !states.is_empty())
        .ok_or(())?;
    let mut coins = HashSet::new();
    for state in states {
        let state = state
            .as_object()
            .filter(|state| state.contains_key("totalVaultEquity"))
            .ok_or(())?;
        // webData3 is a full snapshot. The server omits this optional field for
        // DEXes with no capped assets; HIP-3 names already include their DEX prefix.
        let Some(capped) = state.get("perpsAtOpenInterestCap") else {
            continue;
        };
        for coin in capped.as_array().ok_or(())? {
            let coin = coin
                .as_str()
                .filter(|coin| {
                    !coin.is_empty()
                        && !coin.chars().any(char::is_whitespace)
                        && !coin.starts_with(['@', '#'])
                        && !coin.contains('/')
                })
                .ok_or(())?;
            if let Some((dex, name)) = coin.split_once(':')
                && (dex.is_empty() || name.is_empty() || name.contains(':'))
            {
                return Err(());
            }
            coins.insert(coin.to_string());
        }
    }
    let snapshot = OpenInterestCapSnapshot {
        coins,
        server_time_ms,
        received_at_ms,
    };
    if !snapshot.is_fresh(received_at_ms) {
        return Err(());
    }
    Ok(snapshot)
}

pub(crate) fn ws_open_interest_caps() -> WsStream<OpenInterestCapStreamEvent> {
    Box::pin(iced::stream::channel(10, async move |output| {
        forward_cap_status(get_manager(), output).await;
    }))
}

async fn forward_cap_status(
    (cmd_tx, mut rx): (WsCommandSender, broadcast::Receiver<WsRoutedMessage>),
    mut output: impl futures::Sink<OpenInterestCapStreamEvent> + Unpin,
) {
    let payload = serde_json::json!({
        "method": "subscribe", "subscription": { "type": "webData3", "user": CAP_STATUS_USER }
    });
    if cmd_tx
        .send(WsCommand::Subscribe {
            topic: CAP_STATUS_TOPIC.to_string(),
            payload: payload.clone(),
        })
        .is_err()
    {
        let _ = output.send(OpenInterestCapStreamEvent::Unavailable).await;
        return;
    }
    let _guard = SubscriptionGuard {
        cmd_tx: cmd_tx.clone(),
        subscriptions: vec![(CAP_STATUS_TOPIC.to_string(), payload)],
    };
    let mut deadline = Instant::now() + STALE_AFTER;
    loop {
        let event = tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                let _ = cmd_tx.send(WsCommand::Resubscribe { topic: CAP_STATUS_TOPIC.to_string() });
                OpenInterestCapStreamEvent::Unavailable
            }
            received = rx.recv() => match received {
                Ok(message) if message.channel == "webData3"
                    && message.data.pointer("/userState/user").and_then(Value::as_str) == Some(CAP_STATUS_USER) => {
                    match parse_snapshot(&message.data, crate::ws::now_ms()) {
                        Ok(snapshot) => OpenInterestCapStreamEvent::Snapshot(snapshot),
                        Err(()) => OpenInterestCapStreamEvent::Unavailable,
                    }
                }
                Ok(message) if message.channel == "keroseneDisconnected" => {
                    OpenInterestCapStreamEvent::Unavailable
                }
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    cmd_tx.request_lag_reconnect();
                    OpenInterestCapStreamEvent::Unavailable
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let _ = output.send(OpenInterestCapStreamEvent::Unavailable).await;
                    return;
                }
            }
        };
        deadline = Instant::now() + STALE_AFTER;
        if output.send(event).await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
