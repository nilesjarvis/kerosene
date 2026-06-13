use super::super::super::HYDROMANCER_RECONNECT_DELAY_SECS;
use super::super::{HydromancerCommand, HydromancerRoutedMessage};
use super::coalescer::HydromancerCoalescedSender;
use super::frames::{HydromancerTextFrameKind, parse_hydromancer_text_frame};
use super::messages::{
    broadcast_hydromancer_heartbeat, broadcast_hydromancer_json,
    broadcast_hydromancer_reconnecting, hydromancer_unsubscribe_payload,
};
use super::session::HydromancerSessionState;
use super::subscriptions::{ActiveHydromancerSubscriptions, HydromancerUnsubscribeResult};
use crate::ws::{telemetry_add_hydromancer_rx, telemetry_add_hydromancer_tx};

use futures::{Sink, SinkExt as _};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message as WsMsg;

#[cfg(test)]
mod tests;

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
        HydromancerCommand::Reconnect => true,
        // Shutdown is intercepted by the inner select arm in `task.rs`
        // before this dispatcher is called — but having the variant here
        // keeps the match exhaustive without a wildcard.
        HydromancerCommand::Shutdown => true,
    }
}

pub(super) async fn handle_hydromancer_ws_message<W>(
    msg: WsMsg,
    active_subs: &ActiveHydromancerSubscriptions,
    session: &mut HydromancerSessionState,
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
    coalescer: &mut HydromancerCoalescedSender,
    write: &mut W,
) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    match msg {
        WsMsg::Text(text) => {
            telemetry_add_hydromancer_rx(text.len() as u64);
            handle_hydromancer_text_frame(&text, active_subs, session, msg_tx, coalescer, write)
                .await
        }
        WsMsg::Ping(payload) => {
            let _ = broadcast_hydromancer_heartbeat(msg_tx);
            write.send(WsMsg::Pong(payload)).await.is_err()
        }
        WsMsg::Pong(_) => {
            let _ = broadcast_hydromancer_heartbeat(msg_tx);
            false
        }
        WsMsg::Close(_) => {
            let _ = broadcast_hydromancer_reconnecting(
                msg_tx,
                "stream closed",
                HYDROMANCER_RECONNECT_DELAY_SECS,
            );
            true
        }
        _ => false,
    }
}

async fn handle_hydromancer_text_frame<W>(
    text: &str,
    active_subs: &ActiveHydromancerSubscriptions,
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

    match frame.kind {
        HydromancerTextFrameKind::Connected | HydromancerTextFrameKind::Reconnected => {
            let _ = broadcast_hydromancer_json(msg_tx, frame.json);
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
    write.send(WsMsg::Text(text.into())).await.is_ok()
}
