use self::commands::handle_ws_command;
use self::frames::{WsTextFrame, parse_ws_text_frame};
use self::subscriptions::ActiveWsSubscriptions;
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

use self::coalescer::CoalescedSender;

#[cfg(test)]
mod tests;

const WS_RECONNECT_BASE_DELAY_SECS: u64 = 1;
const WS_RECONNECT_MAX_DELAY_SECS: u64 = 60;
const WS_RECONNECT_RESET_AFTER_SECS: u64 = 30;
// Force a reconnect if no frame (data or pong) has arrived for this long.
// The manager pings every 30s, so 45s = "missed at least one heartbeat round-trip"
// and is the recovery path for half-open sockets left behind by VPN/NAT rebinds,
// where reads silently stall while writes still queue into the kernel buffer.
const WS_READ_STALE_AFTER_SECS: u64 = 45;

fn stale_read_remaining(last_rx_elapsed: Duration) -> Duration {
    Duration::from_secs(WS_READ_STALE_AFTER_SECS).saturating_sub(last_rx_elapsed)
}

fn next_reconnect_delay_secs(current_secs: u64) -> u64 {
    if current_secs < WS_RECONNECT_BASE_DELAY_SECS {
        return WS_RECONNECT_BASE_DELAY_SECS;
    }
    current_secs
        .saturating_mul(2)
        .min(WS_RECONNECT_MAX_DELAY_SECS)
}

fn reconnect_delay_after_disconnect(current_secs: u64, connected_for: Duration) -> (u64, u64) {
    let delay_secs = if connected_for >= Duration::from_secs(WS_RECONNECT_RESET_AFTER_SECS) {
        WS_RECONNECT_BASE_DELAY_SECS
    } else {
        current_secs.clamp(WS_RECONNECT_BASE_DELAY_SECS, WS_RECONNECT_MAX_DELAY_SECS)
    };
    (delay_secs, next_reconnect_delay_secs(delay_secs))
}

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

        tokio::spawn(ws_manager_task(cmd_rx, msg_tx));
        WsManager { cmd_tx, msg_rx }
    });
    (mgr.cmd_tx.clone(), mgr.msg_rx.resubscribe())
}

async fn ws_manager_task(
    mut cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
) {
    let mut active_subs = ActiveWsSubscriptions::default();
    let mut coalescer = CoalescedSender::new(msg_tx);
    use futures::StreamExt as _;
    use futures::future::{Either, select};
    use tokio_tungstenite::tungstenite::Message as WsMsg;

    let mut reconnect_delay_secs = WS_RECONNECT_BASE_DELAY_SECS;

    loop {
        let ws_stream = match tokio_tungstenite::connect_async(WS_URL).await {
            Ok((ws, _)) => ws,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(reconnect_delay_secs)).await;
                reconnect_delay_secs = next_reconnect_delay_secs(reconnect_delay_secs);
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

            // Two competing deadlines drive the inner select: the read
            // watchdog (force reconnect after `WS_READ_STALE_AFTER_SECS` of
            // silence) and the coalescer flush (release pending l2Book
            // frames). Whichever expires first wakes the loop; we then
            // dispatch on which deadline was the cause.
            let coalesce_in = coalescer.next_due();
            let watchdog_wins = match coalesce_in {
                Some(c) => stale_in <= c,
                None => true,
            };
            let deadline = match coalesce_in {
                Some(c) => stale_in.min(c),
                None => stale_in,
            };

            match tokio::time::timeout(deadline, select(cmd_fut, read_fut)).await {
                Err(_) => {
                    if watchdog_wins {
                        // No frame received within the stale window. Force a
                        // reconnect to recover from half-open sockets that a
                        // VPN/NAT rebind has silently abandoned.
                        disconnected = true;
                    } else {
                        coalescer.flush_due();
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

        telemetry_on_disconnect();
        let (delay_secs, next_delay_secs) =
            reconnect_delay_after_disconnect(reconnect_delay_secs, connected_at.elapsed());
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
