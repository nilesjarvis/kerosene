use super::manager::{HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_manager};
use super::parsing::{
    hydromancer_control_message, hydromancer_fill_items, parse_tracked_trade_event,
    tracked_trade_dedupe_key,
};
use super::recent::RecentHydromancerKeys;
use super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerStreamKey, HydromancerWsMessage};
use crate::ws::WsStream;

use futures::SinkExt as _;
use serde_json::Value;
use tokio::sync::broadcast;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Tracked Trade Stream
// ---------------------------------------------------------------------------

fn tracked_trade_subscription(addresses: Vec<String>) -> Option<(String, Value)> {
    if addresses.is_empty() {
        return None;
    }

    Some((
        format!("userFills:{}", addresses.join(",")),
        serde_json::json!({
            "type": "subscribe",
            "subscription": {
                "type": "userFills",
                "addresses": addresses,
                "aggregateByTime": true
            }
        }),
    ))
}

pub fn ws_hydromancer_tracked_trades(
    stream_key: &(HydromancerStreamKey, u64, Vec<String>),
) -> WsStream<HydromancerWsMessage> {
    let manager_key = stream_key.0.clone();
    let addresses = stream_key.2.clone();

    Box::pin(iced::stream::channel(10000, async move |mut output| {
        let Some((topic, payload)) = tracked_trade_subscription(addresses) else {
            return;
        };

        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(manager_key);
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
        let _guard = HydromancerSubscriptionGuard::new(cmd_tx, vec![subscription]);
        let mut seen = RecentHydromancerKeys::new(50_000);

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if let Some(control) =
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                        && output.send(control).await.is_err()
                    {
                        return;
                    }

                    let Some(items) = hydromancer_fill_items(msg.data.as_ref(), "userFills") else {
                        continue;
                    };

                    for item in items {
                        let Some(event) = parse_tracked_trade_event(item) else {
                            continue;
                        };
                        if seen.insert_new(tracked_trade_dedupe_key(&event))
                            && output
                                .send(HydromancerWsMessage::TrackedTrade(event))
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
                        HydromancerWsMessage::Lagged { skipped },
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
