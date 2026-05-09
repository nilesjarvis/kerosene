use self::messages::{
    broadcast_hydromancer_control, broadcast_hydromancer_reconnecting, hydromancer_connect_url,
};
use self::session::HydromancerSessionState;
use self::socket::{handle_hydromancer_command, handle_hydromancer_ws_message};
use self::subscriptions::ActiveHydromancerSubscriptions;
use super::super::super::{telemetry_on_connect, telemetry_on_disconnect};
use super::super::HYDROMANCER_RECONNECT_DELAY_SECS;
use super::{
    HYDROMANCER_MAX_CONNECT_RETRY_SECS, HYDROMANCER_READ_TIMEOUT_SECS, HydromancerCommand,
    HydromancerRoutedMessage,
};
use tokio::sync::{broadcast, mpsc};

mod frames;
mod messages;
mod session;
mod socket;
mod subscriptions;

// ---------------------------------------------------------------------------
// Hydromancer Manager Task
// ---------------------------------------------------------------------------

pub(super) async fn hydromancer_manager_task(
    api_key: String,
    mut cmd_rx: mpsc::UnboundedReceiver<HydromancerCommand>,
    msg_tx: broadcast::Sender<HydromancerRoutedMessage>,
) {
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    let mut retry_delay = 1;
    let mut session = HydromancerSessionState::default();

    use futures::StreamExt as _;
    use futures::future::{Either, select};

    loop {
        while active_subs.is_empty() {
            match cmd_rx.recv().await {
                Some(HydromancerCommand::Subscribe { topic, payload }) => {
                    active_subs.subscribe(topic, payload);
                }
                Some(HydromancerCommand::Unsubscribe { .. } | HydromancerCommand::Reconnect) => {}
                None => return,
            }
        }

        session.begin_connection();
        let _ = broadcast_hydromancer_control(&msg_tx, "connecting", session.connecting_data());

        let url = hydromancer_connect_url(&api_key, session.session_id(), session.last_cursor());

        let ws_stream = match tokio_tungstenite::connect_async(&url).await {
            Ok((ws, _)) => ws,
            Err(e) => {
                let _ = broadcast_hydromancer_reconnecting(&msg_tx, e.to_string(), retry_delay);
                tokio::time::sleep(std::time::Duration::from_secs(retry_delay)).await;
                retry_delay = (retry_delay * 2).min(HYDROMANCER_MAX_CONNECT_RETRY_SECS);
                continue;
            }
        };

        retry_delay = 1;
        telemetry_on_connect();
        let (mut write, mut read) = ws_stream.split();
        let mut disconnected = false;

        while !disconnected {
            let cmd_fut = Box::pin(cmd_rx.recv());
            let read_fut = Box::pin(tokio::time::timeout(
                std::time::Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS),
                read.next(),
            ));

            match select(cmd_fut, read_fut).await {
                Either::Left((Some(cmd), _)) => {
                    disconnected =
                        handle_hydromancer_command(cmd, &mut active_subs, &session, &mut write)
                            .await;
                }
                Either::Left((None, _)) => {
                    disconnected = true;
                }
                Either::Right((Ok(Some(Ok(msg))), _)) => {
                    disconnected = handle_hydromancer_ws_message(
                        msg,
                        &active_subs,
                        &mut session,
                        &msg_tx,
                        &mut write,
                    )
                    .await;
                }
                Either::Right((Ok(Some(Err(e))), _)) => {
                    let _ = broadcast_hydromancer_reconnecting(
                        &msg_tx,
                        e.to_string(),
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    );
                    disconnected = true;
                }
                Either::Right((Ok(None), _)) => {
                    let _ = broadcast_hydromancer_reconnecting(
                        &msg_tx,
                        "stream closed",
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    );
                    disconnected = true;
                }
                Either::Right((Err(_), _)) => {
                    let _ = broadcast_hydromancer_reconnecting(
                        &msg_tx,
                        format!("heartbeat timeout after {}s", HYDROMANCER_READ_TIMEOUT_SECS),
                        HYDROMANCER_RECONNECT_DELAY_SECS,
                    );
                    disconnected = true;
                }
            }
        }

        telemetry_on_disconnect();
        tokio::time::sleep(std::time::Duration::from_secs(
            HYDROMANCER_RECONNECT_DELAY_SECS,
        ))
        .await;
    }
}
