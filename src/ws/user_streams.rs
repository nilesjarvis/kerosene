mod events;
mod model;
mod routing;
mod subscriptions;

use futures::SinkExt as _;
use std::{future::Future, time::Duration};
use tokio::sync::{broadcast, mpsc};

use super::{SubscriptionGuard, WsCommand, get_manager};
use events::parse_user_stream_message;
use routing::normalize_ws_user_address;
use std::fmt;
use subscriptions::build_user_stream_subscriptions;

pub use model::{KeyedUserData, WsUserData};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WsUserDataStreamParams {
    pub address: Option<String>,
    pub dexes: Vec<String>,
    pub include_mids: bool,
}

impl fmt::Debug for WsUserDataStreamParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WsUserDataStreamParams")
            .field("address", &self.address.as_ref().map(|_| "<redacted>"))
            .field("dexes", &self.dexes)
            .field("include_mids", &self.include_mids)
            .finish()
    }
}

impl WsUserDataStreamParams {
    pub fn new(address: Option<String>, dexes: Vec<String>) -> Self {
        Self {
            address,
            dexes,
            include_mids: true,
        }
    }

    pub fn without_mids(address: Option<String>, dexes: Vec<String>) -> Self {
        Self {
            address,
            dexes,
            include_mids: false,
        }
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
    parse_user_stream_routed_message(channel, data, target_addr, mids_addr, include_mids)
        .map(UserStreamReceiveAction::Emit)
        .unwrap_or(UserStreamReceiveAction::Ignore)
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
            match msg_rx.recv().await {
                Ok(msg) => {
                    let action = user_stream_routed_action(
                        msg.channel.as_str(),
                        msg.data.as_ref(),
                        addr.as_deref(),
                        addr.clone(),
                        include_mids,
                    );
                    match action {
                        UserStreamReceiveAction::Emit(update) => {
                            if output.send(update).await.is_err() {
                                return;
                            }
                        }
                        UserStreamReceiveAction::EmitAndReconnect(update) => {
                            if !emit_user_data_after_reconnect(
                                &reconnect_tx,
                                update,
                                |update| async { output.send(update).await.is_ok() },
                                Duration::ZERO,
                            )
                            .await
                            {
                                return;
                            }
                        }
                        UserStreamReceiveAction::Ignore => {}
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    // Falling behind the 10k-frame broadcast buffer means
                    // we lost order/fill/position updates. Trading code
                    // must NOT silently continue from an unknown state —
                    // surface the lag so the downstream handler can force
                    // a full account refresh.
                    let action = user_stream_lagged_action(addr.clone(), skipped);
                    match action {
                        UserStreamReceiveAction::Emit(update) => {
                            if output.send(update).await.is_err() {
                                return;
                            }
                        }
                        UserStreamReceiveAction::EmitAndReconnect(update) => {
                            if !emit_user_data_after_reconnect(
                                &reconnect_tx,
                                update,
                                |update| async { output.send(update).await.is_ok() },
                                Duration::from_secs(2),
                            )
                            .await
                            {
                                return;
                            }
                        }
                        UserStreamReceiveAction::Ignore => {}
                    }
                }
                Err(error) if crate::ws::broadcast_receiver_closed(&error) => {
                    return;
                }
                Err(_error) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }))
}

fn request_user_data_reconnect_after_lag(cmd_tx: &mpsc::UnboundedSender<WsCommand>) -> bool {
    cmd_tx.send(WsCommand::Reconnect).is_ok()
}

async fn emit_user_data_after_reconnect<T, Emit, Fut>(
    cmd_tx: &mpsc::UnboundedSender<WsCommand>,
    update: T,
    emit: Emit,
    pause: Duration,
) -> bool
where
    Emit: FnOnce(T) -> Fut,
    Fut: Future<Output = bool>,
{
    if !request_user_data_reconnect_after_lag(cmd_tx) {
        return false;
    }
    if !emit(update).await {
        return false;
    }
    if !pause.is_zero() {
        tokio::time::sleep(pause).await;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::WsUserData;

    #[test]
    fn opted_out_stream_ignores_broadcast_all_mids_frames() {
        let update = parse_user_stream_routed_message(
            "allMids",
            &serde_json::json!({ "mids": { "BTC": "100" } }),
            Some("0xabc0000000000000000000000000000000000000"),
            Some("0xabc0000000000000000000000000000000000000".to_string()),
            false,
        );

        assert!(update.is_none());
    }

    #[test]
    fn stream_params_debug_redacts_address() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

        let params = WsUserDataStreamParams::without_mids(
            Some(ADDRESS.to_string()),
            vec!["".to_string(), "dex-a".to_string()],
        );
        let rendered = format!("{params:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(ADDRESS), "{rendered}");
        assert!(rendered.contains("dex-a"), "{rendered}");
        assert!(rendered.contains("include_mids: false"), "{rendered}");
    }

    #[test]
    fn opted_in_stream_keeps_all_mids_frames() {
        let Some((source_addr, WsUserData::AllMids(mids))) = parse_user_stream_routed_message(
            "allMids",
            &serde_json::json!({ "mids": { "BTC": "100" } }),
            Some("0xabc0000000000000000000000000000000000000"),
            Some("0xabc0000000000000000000000000000000000000".to_string()),
            true,
        ) else {
            panic!("expected mids update");
        };

        assert_eq!(
            source_addr.as_deref(),
            Some("0xabc0000000000000000000000000000000000000")
        );
        assert_eq!(mids.get("BTC"), Some(&100.0));
    }

    #[test]
    fn normal_user_data_action_does_not_request_reconnect() {
        let action = user_stream_routed_action(
            "allMids",
            &serde_json::json!({ "mids": { "BTC": "100" } }),
            Some("0xabc0000000000000000000000000000000000000"),
            Some("0xabc0000000000000000000000000000000000000".to_string()),
            true,
        );

        assert!(!action.should_reconnect_after_emit());
        let UserStreamReceiveAction::Emit((source_addr, WsUserData::AllMids(mids))) = action else {
            panic!("expected normal mids update");
        };
        assert_eq!(
            source_addr.as_deref(),
            Some("0xabc0000000000000000000000000000000000000")
        );
        assert_eq!(mids.get("BTC"), Some(&100.0));
    }

    #[test]
    fn lagged_user_data_action_requests_reconnect() {
        let action = user_stream_lagged_action(
            Some("0xabc0000000000000000000000000000000000000".to_string()),
            7,
        );

        assert!(action.should_reconnect_after_emit());
        let UserStreamReceiveAction::EmitAndReconnect((
            source_addr,
            WsUserData::Lagged { skipped },
        )) = action
        else {
            panic!("expected lagged reconnect update");
        };
        assert_eq!(
            source_addr.as_deref(),
            Some("0xabc0000000000000000000000000000000000000")
        );
        assert_eq!(skipped, 7);
    }

    #[test]
    fn user_data_lag_requests_shared_ws_reconnect() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        assert!(request_user_data_reconnect_after_lag(&cmd_tx));
        assert!(matches!(cmd_rx.try_recv().unwrap(), WsCommand::Reconnect));
    }

    #[tokio::test]
    async fn lag_emit_requests_reconnect_before_downstream_send_failure() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        let emitted = emit_user_data_after_reconnect(
            &cmd_tx,
            (None::<String>, WsUserData::Lagged { skipped: 7 }),
            |_update| async { false },
            Duration::ZERO,
        )
        .await;

        assert!(!emitted);
        assert!(matches!(cmd_rx.try_recv().unwrap(), WsCommand::Reconnect));
    }
}
