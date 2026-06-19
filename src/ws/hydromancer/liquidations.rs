use super::manager::{HydromancerCommand, HydromancerSubscriptionGuard, get_hydromancer_manager};
use super::parsing::{
    hydromancer_control_message, hydromancer_fill_items, liquidation_dedupe_key,
    parse_liquidation_event,
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
// Liquidation Stream
// ---------------------------------------------------------------------------

fn liquidation_subscription() -> (String, Value) {
    (
        "liquidationFills".to_string(),
        serde_json::json!({
            "type": "subscribe",
            "subscription": {
                "type": "liquidationFills"
            }
        }),
    )
}

pub fn ws_hydromancer_liquidations(
    stream_key: &(HydromancerStreamKey, u64),
) -> WsStream<HydromancerWsMessage> {
    let manager_key = stream_key.0.clone();
    Box::pin(iced::stream::channel(10000, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_hydromancer_manager(manager_key);
        let (topic, payload) = liquidation_subscription();
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
        let mut seen = RecentHydromancerKeys::new(20_000);

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if let Some(control) =
                        hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                        && output.send(control).await.is_err()
                    {
                        return;
                    }

                    let Some(items) = hydromancer_fill_items(msg.data.as_ref(), "liquidationFills")
                    else {
                        continue;
                    };

                    for item in items {
                        let Some(event) = parse_liquidation_event(item) else {
                            continue;
                        };
                        if seen.insert_new(liquidation_dedupe_key(&event))
                            && output
                                .send(HydromancerWsMessage::Event(event))
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
