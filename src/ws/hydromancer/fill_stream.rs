use super::manager::{
    HydromancerCommand, HydromancerCommandSender, HydromancerRoutedMessage,
    HydromancerSubscriptionGuard,
};
use super::parsing::{
    hydromancer_control_message, hydromancer_fill_items, liquidation_dedupe_key,
    parse_liquidation_event, parse_tracked_trade_event, tracked_trade_dedupe_key,
};
use super::recent::RecentHydromancerKeys;
use super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerWsMessage};

use futures::{Sink, SinkExt as _};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FillFeed {
    Liquidations,
    TrackedTrades,
}

impl FillFeed {
    fn channel_and_capacity(self) -> (&'static str, usize) {
        match self {
            Self::Liquidations => ("liquidationFills", 20_000),
            Self::TrackedTrades => ("userFills", 50_000),
        }
    }

    fn parse_with_key(self, item: &Value) -> Option<(String, HydromancerWsMessage)> {
        match self {
            Self::Liquidations => {
                let event = parse_liquidation_event(item)?;
                Some((
                    liquidation_dedupe_key(&event),
                    HydromancerWsMessage::Event(event),
                ))
            }
            Self::TrackedTrades => {
                let event = parse_tracked_trade_event(item)?;
                Some((
                    tracked_trade_dedupe_key(&event),
                    HydromancerWsMessage::TrackedTrade(event),
                ))
            }
        }
    }
}

pub(super) async fn forward_fill_stream(
    feed: FillFeed,
    (topic, payload): (String, Value),
    (cmd_tx, mut msg_rx): (
        HydromancerCommandSender,
        broadcast::Receiver<HydromancerRoutedMessage>,
    ),
    mut output: impl Sink<HydromancerWsMessage> + Unpin,
) {
    let subscription = (topic.clone(), payload.clone());
    if cmd_tx
        .send(HydromancerCommand::Subscribe { topic, payload })
        .is_err()
    {
        return;
    }
    let reconnect_tx = cmd_tx.clone();
    let _guard = HydromancerSubscriptionGuard::new(cmd_tx, vec![subscription]);
    let (channel, capacity) = feed.channel_and_capacity();
    let mut seen = RecentHydromancerKeys::new(capacity);

    loop {
        match msg_rx.recv().await {
            Ok(msg) => {
                if let Some(control) = hydromancer_control_message(&msg.msg_type, msg.data.as_ref())
                    && output.send(control).await.is_err()
                {
                    return;
                }

                let Some(items) = hydromancer_fill_items(msg.data.as_ref(), channel) else {
                    continue;
                };

                for item in items {
                    let Some((key, event)) = feed.parse_with_key(item) else {
                        continue;
                    };
                    if seen.insert_new(key) && output.send(event).await.is_err() {
                        return;
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                if !super::emit_after_reconnect(
                    || reconnect_tx.request_lag_reconnect(),
                    HydromancerWsMessage::Lagged { skipped },
                    |event| async { output.send(event).await.is_ok() },
                    Duration::from_secs(HYDROMANCER_RECONNECT_DELAY_SECS),
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
                tokio::time::sleep(Duration::from_secs(HYDROMANCER_RECONNECT_DELAY_SECS)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests;
