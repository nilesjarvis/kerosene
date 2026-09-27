use super::recovery::emit_after_reconnect;
mod events;
mod model;
mod routing;
mod subscriptions;

use futures::SinkExt as _;
use std::{future::Future, time::Duration};
use tokio::sync::broadcast;
#[cfg(test)]
use tokio::sync::mpsc;

use super::{SubscriptionGuard, WsCommand, WsCommandSender, get_manager};
use events::parse_user_stream_message;
use routing::{matching_user_payload_address, normalize_ws_user_address};
use std::fmt;
use subscriptions::build_user_stream_subscriptions;

pub use model::{KeyedUserData, WsUserData};

/// Identifies which feature consumes a user-data stream.
///
/// This is part of the stream's subscription identity (it is included in
/// `Hash`/`Eq`) so that two features watching the SAME address — e.g. a
/// wallet-detail window and a wallet-cluster member — produce distinct iced
/// subscriptions instead of colliding on a single recipe hash. `Subscription::map`
/// does not change a subscription's identity, so without this discriminant iced
/// would treat the two `without_mids` streams as one recipe and silently drop
/// one consumer's updates. The purpose is NOT sent over the wire; the actual
/// topic subscription is deduplicated by the ws manager.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WsUserDataStreamPurpose {
    Account,
    WalletDetail,
    WalletCluster,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WsUserDataStreamParams {
    pub address: Option<String>,
    pub dexes: Vec<String>,
    pub include_mids: bool,
    pub purpose: WsUserDataStreamPurpose,
}

impl fmt::Debug for WsUserDataStreamParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WsUserDataStreamParams")
            .field("address", &self.address.as_ref().map(|_| "<redacted>"))
            .field("dexes", &self.dexes)
            .field("include_mids", &self.include_mids)
            .field("purpose", &self.purpose)
            .finish()
    }
}

impl WsUserDataStreamParams {
    pub fn new(address: Option<String>, dexes: Vec<String>) -> Self {
        Self {
            address,
            dexes,
            include_mids: true,
            purpose: WsUserDataStreamPurpose::Account,
        }
    }

    pub fn without_mids(address: Option<String>, dexes: Vec<String>) -> Self {
        Self {
            address,
            dexes,
            include_mids: false,
            purpose: WsUserDataStreamPurpose::Account,
        }
    }

    /// Sets the consuming feature, distinguishing otherwise-identical streams
    /// (same address/dexes/mids) so iced keeps both alive.
    pub fn with_purpose(mut self, purpose: WsUserDataStreamPurpose) -> Self {
        self.purpose = purpose;
        self
    }
}

fn parse_user_stream_routed_message(
    channel: &str,
    data: &serde_json::Value,
    target_addr: Option<&str>,
    mids_addr: Option<String>,
    include_mids: bool,
) -> Option<KeyedUserData> {
    if !include_mids && channel == "allMids" {
        return None;
    }

    parse_user_stream_message(
        channel,
        data,
        target_addr,
        include_mids.then_some(mids_addr).flatten(),
    )
}

enum UserStreamReceiveAction {
    Emit(KeyedUserData),
    EmitAndReconnect(KeyedUserData),
    Ignore,
}

impl UserStreamReceiveAction {
    async fn emit<Emit, Fut>(self, cmd_tx: &WsCommandSender, emit: Emit, pause: Duration) -> bool
    where
        Emit: FnOnce(KeyedUserData) -> Fut,
        Fut: Future<Output = bool>,
    {
        match self {
            Self::Emit(update) => emit(update).await,
            Self::EmitAndReconnect(update) => {
                emit_after_reconnect(|| cmd_tx.request_lag_reconnect(), update, emit, pause).await
            }
            Self::Ignore => true,
        }
    }

    #[cfg(test)]
    fn should_reconnect_after_emit(&self) -> bool {
        matches!(self, Self::EmitAndReconnect(_))
    }
}

fn user_stream_routed_action(
    channel: &str,
    data: &serde_json::Value,
    target_addr: Option<&str>,
    mids_addr: Option<String>,
    include_mids: bool,
) -> UserStreamReceiveAction {
    if let Some(update) =
        parse_user_stream_routed_message(channel, data, target_addr, mids_addr, include_mids)
    {
        return UserStreamReceiveAction::Emit(update);
    }

    if channel == "spotState"
        && let Some(source_addr) = matching_user_payload_address(data, target_addr)
    {
        // A correctly routed spotState frame that fails schema parsing must
        // not be silently ignored: balances are now unknown. Force the same
        // reconciliation/reconnect path used for an explicitly lagged stream.
        return UserStreamReceiveAction::EmitAndReconnect((
            Some(source_addr),
            WsUserData::Lagged { skipped: 1 },
        ));
    }

    UserStreamReceiveAction::Ignore
}

fn user_stream_lagged_action(addr: Option<String>, skipped: u64) -> UserStreamReceiveAction {
    UserStreamReceiveAction::EmitAndReconnect((addr, WsUserData::Lagged { skipped }))
}

pub fn ws_user_data_stream(
    params: &WsUserDataStreamParams,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = KeyedUserData> + Send>> {
    let addr = params
        .address
        .as_deref()
        .and_then(normalize_ws_user_address);
    let dexes = params.dexes.clone();
    let include_mids = params.include_mids;

    Box::pin(iced::stream::channel(20, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_manager();

        let mut subscriptions = Vec::new();
        for (topic, payload) in
            build_user_stream_subscriptions(addr.as_deref(), &dexes, include_mids)
        {
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
            subscriptions.push(subscription);
        }

        let reconnect_tx = cmd_tx.clone();
        let _guard = SubscriptionGuard {
            cmd_tx,
            subscriptions,
        };

        loop {
            let (action, pause) = match msg_rx.recv().await {
                Ok(msg) => (
                    user_stream_routed_action(
                        msg.channel.as_str(),
                        msg.data.as_ref(),
                        addr.as_deref(),
                        addr.clone(),
                        include_mids,
                    ),
                    Duration::ZERO,
                ),
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    // Lost account updates require reconciliation and transport
                    // recovery before more data can be trusted.
                    (
                        user_stream_lagged_action(addr.clone(), skipped),
                        Duration::from_secs(2),
                    )
                }
                Err(error) if crate::ws::broadcast_receiver_closed(&error) => return,
                Err(_error) => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };
            if !action
                .emit(
                    &reconnect_tx,
                    |update| async { output.send(update).await.is_ok() },
                    pause,
                )
                .await
            {
                return;
            }
        }
    }))
}

#[cfg(test)]
mod tests;
