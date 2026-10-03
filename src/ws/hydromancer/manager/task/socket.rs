use super::super::super::HYDROMANCER_RECONNECT_DELAY_SECS;
use super::super::super::capacity::{has_market_fallback, is_capacity_error, is_capacity_message};
use super::super::{HydromancerCommand, HydromancerRoutedMessage};
use super::coalescer::HydromancerCoalescedSender;
use super::frames::{HydromancerTextFrameKind, parse_hydromancer_text_frame};
use super::messages::{
    broadcast_hydromancer_heartbeat, broadcast_hydromancer_json,
    broadcast_hydromancer_reconnecting, hydromancer_unsubscribe_payload,
};
use super::session::HydromancerSessionState;
use super::subscriptions::{ActiveHydromancerSubscriptions, HydromancerUnsubscribeResult};
use crate::network_activity::{Provider, record_ws_frame};
use crate::ws::{telemetry_add_hydromancer_rx, telemetry_add_hydromancer_tx};

use futures::{Sink, SinkExt as _};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message as WsMsg;

#[cfg(test)]
mod tests;

#[cfg(not(test))]
const HYDROMANCER_WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
#[cfg(test)]
const HYDROMANCER_WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(20);

// ---------------------------------------------------------------------------
// Connected Socket Event Handling
// ---------------------------------------------------------------------------

pub(super) async fn handle_hydromancer_command<W>(
    cmd: HydromancerCommand,
    active_subs: &mut ActiveHydromancerSubscriptions,
    session: &HydromancerSessionState,
    write: &mut W,
) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    match cmd {
        HydromancerCommand::Subscribe { topic, payload } => {
            let new_payload = active_subs.subscribe(topic, payload);
            if session.connection_ready()
                && let Some(payload) = new_payload
            {
                return !send_text(write, payload.to_string()).await;
            }
            false
        }
        HydromancerCommand::Unsubscribe { topic, payload } => {
            match active_subs.unsubscribe(topic, payload) {
                HydromancerUnsubscribeResult::Removed {
                    payload,
                    became_empty,
                } => {
                    let payload = hydromancer_unsubscribe_payload(&payload);
                    !send_text(write, payload.to_string()).await || became_empty
                }
                HydromancerUnsubscribeResult::StillActive
                | HydromancerUnsubscribeResult::Missing => false,
            }
        }
        HydromancerCommand::Resubscribe { topic } => {
            if session.connection_ready()
                && let Some(payload) = active_subs.resubscribe(&topic)
            {
                if !send_text(write, hydromancer_unsubscribe_payload(&payload).to_string()).await {
                    return true;
                }
                return !send_text(write, payload.to_string()).await;
            }
            false
        }
        HydromancerCommand::Reconnect => true,
        // Shutdown is intercepted by the inner select arm in `task.rs`
        // before this dispatcher is called — but having the variant here
        // keeps the match exhaustive without a wildcard.
        HydromancerCommand::Shutdown => true,
    }
}

pub(super) async fn handle_hydromancer_ws_message<W>(
    msg: WsMsg,
    active_subs: &mut ActiveHydromancerSubscriptions,
    session: &mut HydromancerSessionState,
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
    coalescer: &mut HydromancerCoalescedSender,
    write: &mut W,
) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    record_ws_frame(Provider::Hydromancer, &msg, true);
    match msg {
        WsMsg::Text(text) => {
            telemetry_add_hydromancer_rx(text.len() as u64);
            handle_hydromancer_text_frame(&text, active_subs, session, msg_tx, coalescer, write)
                .await
        }
        WsMsg::Ping(payload) => {
            let _ = broadcast_hydromancer_heartbeat(msg_tx);
            !send_with_timeout(write, WsMsg::Pong(payload)).await
        }
        WsMsg::Pong(_) => {
            let _ = broadcast_hydromancer_heartbeat(msg_tx);
            false
        }
        WsMsg::Close(frame) => {
            let capacity_exceeded = frame
                .as_ref()
                .is_some_and(|frame| is_capacity_error(&frame.reason));
            if capacity_exceeded {
                active_subs.fallback_market_streams();
                // A closed socket cannot unsubscribe the retired topics. Do not
                // resume a server session that still owns those subscriptions.
                *session = HydromancerSessionState::default();
            }
            let _ = broadcast_hydromancer_reconnecting(
                msg_tx,
                if capacity_exceeded {
                    "Hydromancer subscription capacity exceeded"
                } else {
                    "stream closed"
                },
                HYDROMANCER_RECONNECT_DELAY_SECS,
            );
            true
        }
        _ => false,
    }
}

async fn handle_hydromancer_text_frame<W>(
    text: &str,
    active_subs: &mut ActiveHydromancerSubscriptions,
    session: &mut HydromancerSessionState,
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
    coalescer: &mut HydromancerCoalescedSender,
    write: &mut W,
) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    let Some(frame) = parse_hydromancer_text_frame(text) else {
        return false;
    };
    let frame_action = session.apply_text_frame(&frame);

    let msg_type = frame
        .json
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if is_capacity_message(msg_type, &frame.json) {
        let retired = active_subs.fallback_market_streams();
        let market_rejection = has_market_fallback(&frame.json);
        if !retired.is_empty() {
            // Free capacity before retrying priority feeds. Keep their socket
            // and resume cursor alive when the provider permits it.
            coalescer.flush_all();
            for payload in retired {
                if !send_text(write, hydromancer_unsubscribe_payload(&payload).to_string()).await {
                    *session = HydromancerSessionState::default();
                    return true;
                }
            }
            if !market_rejection {
                // Legacy errors do not identify the rejected topic. Start a
                // clean session with priority feeds only: queued rejections
                // from retired charts must not poison those feeds, and a
                // required subscription may need to be retried as well.
                *session = HydromancerSessionState::default();
                let _ = broadcast_hydromancer_reconnecting(
                    msg_tx,
                    "prioritizing required feeds after capacity limit",
                    HYDROMANCER_RECONNECT_DELAY_SECS,
                );
                return true;
            }
            // A charts-only workspace should release its now-unused socket.
            return active_subs.is_empty();
        }
        if market_rejection {
            // Late rejections for retired charts must not disconnect fill feeds.
            return false;
        }
    }

    match frame.kind {
        HydromancerTextFrameKind::Connected | HydromancerTextFrameKind::Reconnected => {
            let _ = broadcast_hydromancer_json(msg_tx, frame.json);
            active_subs.notify_market_fallback();
            if !frame_action.resend_subscriptions {
                return false;
            }
            for payload in active_subs.payloads() {
                if !send_text(write, payload.to_string()).await {
                    return true;
                }
            }
            false
        }
        HydromancerTextFrameKind::Ping => {
            let disconnected = if frame_action.send_pong {
                !send_text(write, serde_json::json!({ "type": "pong" }).to_string()).await
            } else {
                false
            };
            let _ = broadcast_hydromancer_json(msg_tx, frame.json);
            disconnected
        }
        HydromancerTextFrameKind::Other => {
            coalescer.submit_json(frame.json);
            false
        }
    }
}

async fn send_text<W>(write: &mut W, text: String) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    telemetry_add_hydromancer_tx(text.len() as u64);
    send_with_timeout(write, WsMsg::Text(text.into())).await
}

async fn send_with_timeout<W>(write: &mut W, message: WsMsg) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    record_ws_frame(Provider::Hydromancer, &message, false);
    let mut send = std::pin::pin!(write.send(message));
    let first_poll = futures::future::poll_fn(|cx| {
        std::task::Poll::Ready(std::future::Future::poll(send.as_mut(), cx))
    })
    .await;
    if let std::task::Poll::Ready(result) = first_poll {
        return result.is_ok();
    }
    tokio::time::timeout(HYDROMANCER_WRITE_TIMEOUT, send)
        .await
        .is_ok_and(|result| result.is_ok())
}
