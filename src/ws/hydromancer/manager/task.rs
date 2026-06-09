use self::coalescer::HydromancerCoalescedSender;
use self::lifecycle::{
    HydromancerTaskControlFlow, drain_pending_hydromancer_shutdown,
    handle_preconnect_hydromancer_command, hydromancer_sleep_or_shutdown,
};
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
    HydromancerRoutedMessage, hydromancer_read_remaining,
};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use zeroize::Zeroizing;

mod coalescer;
mod frames;
mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
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
    let api_key = Zeroizing::new(api_key);
    let mut active_subs = ActiveHydromancerSubscriptions::default();
    let mut coalescer = HydromancerCoalescedSender::new(msg_tx.clone());
    let mut retry_delay = 1;
    let mut session = HydromancerSessionState::default();

    use futures::StreamExt as _;
    use futures::future::{Either, select};

    'manager: loop {
        while active_subs.is_empty() {
            match cmd_rx.recv().await {
                Some(cmd) => {
                    if handle_preconnect_hydromancer_command(cmd, &mut active_subs)
                        == HydromancerTaskControlFlow::Shutdown
                    {
                        return;
                    }
                }
                None => return,
            }
        }

        // A Shutdown can be queued by key rotation while this task is between
        // reconnect sleeps and the next connect attempt. Drain it before
        // constructing another old-key URL.
        if drain_pending_hydromancer_shutdown(&mut cmd_rx, &mut active_subs)
            == HydromancerTaskControlFlow::Shutdown
        {
            return;
        }
        if active_subs.is_empty() {
            continue;
        }

        session.begin_connection();
        let _ = broadcast_hydromancer_control(&msg_tx, "connecting", session.connecting_data());

        let url = hydromancer_connect_url(&api_key, session.session_id(), session.last_cursor());

        let connect_fut = Box::pin(tokio_tungstenite::connect_async(&url));
        let cmd_fut = Box::pin(cmd_rx.recv());
        let connect_result = match select(connect_fut, cmd_fut).await {
            Either::Left((connect_result, pending_cmd)) => {
                drop(pending_cmd);
                connect_result
            }
            Either::Right((cmd, pending_connect)) => {
                // A rotation/clear command must be able to cancel an in-flight
                // connect attempt before the old key establishes another
                // socket. Other commands are folded into subscription state and
                // the outer loop starts a fresh connect attempt if still needed.
                drop(pending_connect);
                match cmd {
                    Some(cmd) => {
                        if handle_preconnect_hydromancer_command(cmd, &mut active_subs)
                            == HydromancerTaskControlFlow::Shutdown
                        {
                            return;
                        }
                        continue 'manager;
                    }
                    None => return,
                }
            }
        };

        let ws_stream = match connect_result {
            Ok((ws, _)) => ws,
            Err(e) => {
                let _ = broadcast_hydromancer_reconnecting(&msg_tx, e.to_string(), retry_delay);
                if hydromancer_sleep_or_shutdown(
                    &mut cmd_rx,
                    &mut active_subs,
                    Duration::from_secs(retry_delay),
                )
                .await
                    == HydromancerTaskControlFlow::Shutdown
                {
                    return;
                }
                retry_delay = (retry_delay * 2).min(HYDROMANCER_MAX_CONNECT_RETRY_SECS);
                continue 'manager;
            }
        };

        retry_delay = 1;
        telemetry_on_connect();
        let (mut write, mut read) = ws_stream.split();
        let mut disconnected = false;
        let mut last_rx_at = Instant::now();

        while !disconnected {
            let cmd_fut = Box::pin(cmd_rx.recv());
            let read_timeout = coalescer
                .next_due()
                .map(|due| due.min(hydromancer_read_remaining(last_rx_at.elapsed())))
                .unwrap_or_else(|| hydromancer_read_remaining(last_rx_at.elapsed()));
            let read_fut = Box::pin(tokio::time::timeout(read_timeout, read.next()));

            match select(cmd_fut, read_fut).await {
                Either::Left((Some(HydromancerCommand::Shutdown), _)) => {
                    // Rotation / clear path: stop the task so the owned
                    // `api_key` String is dropped instead of resident for
                    // the process lifetime.
                    return;
                }
                Either::Left((Some(cmd), _)) => {
                    disconnected =
                        handle_hydromancer_command(cmd, &mut active_subs, &session, &mut write)
                            .await;
                }
                Either::Left((None, _)) => {
                    disconnected = true;
                }
                Either::Right((Ok(Some(Ok(msg))), _)) => {
                    last_rx_at = Instant::now();
                    disconnected = handle_hydromancer_ws_message(
                        msg,
                        &active_subs,
                        &mut session,
                        &msg_tx,
                        &mut coalescer,
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
                    coalescer.flush_due();
                    if hydromancer_read_remaining(last_rx_at.elapsed()).is_zero() {
                        let _ = broadcast_hydromancer_reconnecting(
                            &msg_tx,
                            format!("heartbeat timeout after {}s", HYDROMANCER_READ_TIMEOUT_SECS),
                            HYDROMANCER_RECONNECT_DELAY_SECS,
                        );
                        disconnected = true;
                    }
                }
            }
        }

        coalescer.flush_all();
        telemetry_on_disconnect();
        if hydromancer_sleep_or_shutdown(
            &mut cmd_rx,
            &mut active_subs,
            Duration::from_secs(HYDROMANCER_RECONNECT_DELAY_SECS),
        )
        .await
            == HydromancerTaskControlFlow::Shutdown
        {
            return;
        }
    }
}
