use self::commands::handle_ws_command;
use self::frames::{WsTextFrame, parse_ws_text_frame};
use self::subscriptions::ActiveWsSubscriptions;
use self::timing::{EXCHANGE_WS_RECONNECT_POLICY, read_loop_timeout, stale_read_remaining};
use super::WS_URL;
use super::telemetry::{
    now_ms, telemetry_add_rx, telemetry_add_tx, telemetry_mark_ws_ping_start, telemetry_on_connect,
    telemetry_on_disconnect, telemetry_update_api_latency,
    telemetry_update_ws_latency_from_ping_start,
};
use futures::SinkExt as _;
use serde_json::Value;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

mod coalescer;
mod commands;
mod frames;
mod subscriptions;
mod timing;

use self::coalescer::CoalescedSender;
#[cfg(test)]
use self::timing::{ReconnectPolicy, WS_READ_STALE_AFTER_SECS};

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Global Multiplexer Setup
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WsRoutedMessage {
    pub channel: String,
    pub data: Arc<Value>,
}

#[derive(Debug)]
pub enum WsCommand {
    Subscribe { topic: String, payload: Value },
    Unsubscribe { topic: String },
    Ping,
}

struct WsManager {
    cmd_tx: mpsc::UnboundedSender<WsCommand>,
    msg_rx: broadcast::Receiver<WsRoutedMessage>,
}

static WS_MANAGER: OnceLock<WsManager> = OnceLock::new();

pub(crate) fn get_manager() -> (
    mpsc::UnboundedSender<WsCommand>,
    broadcast::Receiver<WsRoutedMessage>,
) {
    let mgr = WS_MANAGER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(10000);

        let ping_tx = cmd_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                if ping_tx.send(WsCommand::Ping).is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            loop {
                let start_time = now_ms();
                let client = crate::api::CLIENT.clone();
                let req_payload = serde_json::json!({ "type": "ping" });

                if let Ok(resp) = client
                    .post(crate::api::API_URL)
                    .json(&req_payload)
                    .send()
                    .await
                    && resp.status().is_success()
                {
                    let latency = now_ms().saturating_sub(start_time);
                    telemetry_update_api_latency(latency);
                }

                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        tokio::spawn(ws_manager_task(WS_URL.to_string(), cmd_rx, msg_tx));
        WsManager { cmd_tx, msg_rx }
    });
    (mgr.cmd_tx.clone(), mgr.msg_rx.resubscribe())
}

pub(super) async fn ws_manager_task(
    ws_url: String,
    mut cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
) {
    let mut active_subs = ActiveWsSubscriptions::default();
    let mut coalescer = CoalescedSender::new(msg_tx);
    use futures::StreamExt as _;
    use futures::future::{Either, select};
    use tokio_tungstenite::tungstenite::Message as WsMsg;

    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let mut reconnect_delay_secs = policy.base_delay_secs;

    loop {
        let ws_stream = match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((ws, _)) => ws,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(reconnect_delay_secs)).await;
                reconnect_delay_secs = policy.next_delay(reconnect_delay_secs);
                continue;
            }
        };
        telemetry_on_connect();
        let connected_at = Instant::now();

        let (mut write, mut read) = ws_stream.split();
        let mut disconnected = false;
        let mut last_rx_at = Instant::now();

        for payload in active_subs.payloads() {
            let text = payload.to_string();
            telemetry_add_tx(text.len() as u64);
            if write.send(WsMsg::Text(text.into())).await.is_err() {
                disconnected = true;
                break;
            }
        }

        while !disconnected {
            let cmd_fut = Box::pin(cmd_rx.recv());
            let read_fut = Box::pin(read.next());
            let stale_in = stale_read_remaining(last_rx_at.elapsed());
            let timeout_in = read_loop_timeout(stale_in, coalescer.next_due());

            match tokio::time::timeout(timeout_in, select(cmd_fut, read_fut)).await {
                Err(_) => {
                    coalescer.flush_due();

                    // No frame received within the stale window. Force a
                    // reconnect to recover from half-open sockets that a
                    // VPN/NAT rebind has silently abandoned.
                    if stale_read_remaining(last_rx_at.elapsed()).is_zero() {
                        disconnected = true;
                    }
                }
                Ok(Either::Left((Some(cmd), _))) => {
                    let action = handle_ws_command(&mut active_subs, cmd);
                    if action.mark_ping_start {
                        telemetry_mark_ws_ping_start();
                    }
                    if let Some(payload) = action.outbound_payload {
                        let text = payload.to_string();
                        telemetry_add_tx(text.len() as u64);
                        if write.send(WsMsg::Text(text.into())).await.is_err()
                            && action.disconnect_on_send_error
                        {
                            disconnected = true;
                        }
                    }
                }
                Ok(Either::Left((None, _))) => {
                    disconnected = true;
                }
                Ok(Either::Right((msg_opt, _))) => match msg_opt {
                    Some(Ok(WsMsg::Text(text))) => {
                        last_rx_at = Instant::now();
                        telemetry_add_rx(text.len() as u64);
                        match parse_ws_text_frame(&text) {
                            WsTextFrame::Pong => {
                                telemetry_update_ws_latency_from_ping_start();
                            }
                            WsTextFrame::Data { channel, data } => {
                                coalescer.submit(channel, Arc::new(data));
                            }
                            WsTextFrame::Ignored => {}
                        }
                    }
                    Some(Ok(_)) => {
                        last_rx_at = Instant::now();
                    }
                    Some(Err(_)) | None => {
                        disconnected = true;
                    }
                },
            }
        }

        coalescer.flush_all();
        telemetry_on_disconnect();
        let (delay_secs, next_delay_secs) =
            policy.after_disconnect(reconnect_delay_secs, connected_at.elapsed());
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        reconnect_delay_secs = next_delay_secs;
    }
}

pub(crate) struct SubscriptionGuard {
    pub(crate) cmd_tx: mpsc::UnboundedSender<WsCommand>,
    pub(crate) topics: Vec<String>,
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        for topic in &self.topics {
            let _ = self.cmd_tx.send(WsCommand::Unsubscribe {
                topic: topic.clone(),
            });
        }
    }
}
