use self::commands::handle_ws_command;
use self::frames::{WsTextFrame, parse_ws_text_frame};
use self::subscriptions::ActiveWsSubscriptions;
use self::timing::{
    EXCHANGE_WS_RECONNECT_POLICY, ReconnectPolicy, WS_CONNECT_TIMEOUT_SECS, read_loop_timeout,
    stale_read_remaining,
};
use super::WS_URL;
use super::connect::{ConnectAttempt, connect_with_timeout};
#[cfg(not(test))]
use super::telemetry::{now_ms, telemetry_update_api_latency};
use super::telemetry::{
    telemetry_add_rx, telemetry_add_tx, telemetry_mark_ws_ping_start, telemetry_on_connect,
    telemetry_on_disconnect, telemetry_update_ws_latency_from_ping_start,
};
use futures::{Sink, SinkExt as _};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message as WsMsg;

mod coalescer;
mod commands;
mod frames;
mod subscriptions;
mod timing;

use self::coalescer::CoalescedSender;
#[cfg(test)]
use self::timing::WS_READ_STALE_AFTER_SECS;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Global Multiplexer Setup
// ---------------------------------------------------------------------------

#[cfg(not(test))]
const API_LATENCY_PROBE_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(test)]
const API_LATENCY_PROBE_INTERVAL: Duration = Duration::from_millis(50);
#[cfg(not(test))]
const WS_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const WS_WRITE_TIMEOUT: Duration = Duration::from_millis(20);

#[derive(Clone)]
pub struct WsRoutedMessage {
    pub channel: String,
    pub data: Arc<Value>,
}

impl fmt::Debug for WsRoutedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WsRoutedMessage")
            .field("channel", &redacted_ws_topic_debug_value(&self.channel))
            .field("data", &redacted_ws_value(&self.data))
            .finish()
    }
}

pub enum WsCommand {
    Subscribe { topic: String, payload: Value },
    Unsubscribe { topic: String, payload: Value },
    Ping,
    Reconnect,
}

impl fmt::Debug for WsCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Subscribe { topic, payload } => f
                .debug_struct("Subscribe")
                .field("topic", &redacted_ws_topic_debug_value(topic))
                .field("payload", &redacted_ws_value(payload))
                .finish(),
            Self::Unsubscribe { topic, payload } => f
                .debug_struct("Unsubscribe")
                .field("topic", &redacted_ws_topic_debug_value(topic))
                .field("payload", &redacted_ws_value(payload))
                .finish(),
            Self::Ping => f.write_str("Ping"),
            Self::Reconnect => f.write_str("Reconnect"),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RedactedWsValue<'a>(&'a Value);

impl fmt::Debug for RedactedWsValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = self.0.get("method").and_then(Value::as_str);
        let subscription_type = self
            .0
            .pointer("/subscription/type")
            .and_then(Value::as_str)
            .or_else(|| self.0.get("type").and_then(Value::as_str));

        f.debug_struct("WsJson")
            .field("method", &method)
            .field("subscription_type", &subscription_type)
            .field("payload", &"<redacted>")
            .finish()
    }
}

pub(super) fn redacted_ws_value(value: &Value) -> RedactedWsValue<'_> {
    RedactedWsValue(value)
}

pub(super) fn redacted_ws_topic_debug_value(topic: &str) -> &str {
    let lower = topic.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.contains(":0x") {
        "<redacted>"
    } else {
        topic
    }
}

struct WsManager {
    cmd_tx: WsCommandSender,
    msg_rx: broadcast::Receiver<WsRoutedMessage>,
}

static WS_MANAGER: OnceLock<WsManager> = OnceLock::new();

#[derive(Clone, Default)]
struct WsReconnectGate(Arc<AtomicBool>);

impl WsReconnectGate {
    fn request_lag_reconnect(&self, cmd_tx: &mpsc::UnboundedSender<WsCommand>) -> bool {
        if self.0.swap(true, Ordering::AcqRel) {
            return true;
        }
        if cmd_tx.send(WsCommand::Reconnect).is_err() {
            self.0.store(false, Ordering::Release);
            return false;
        }
        true
    }

    fn note_dequeued(&self, command: &WsCommand) {
        if matches!(command, WsCommand::Reconnect) {
            self.0.store(false, Ordering::Release);
        }
    }
}

#[derive(Clone)]
pub(crate) struct WsCommandSender {
    inner: mpsc::UnboundedSender<WsCommand>,
    reconnect_gate: WsReconnectGate,
}

impl WsCommandSender {
    fn new(inner: mpsc::UnboundedSender<WsCommand>, reconnect_gate: WsReconnectGate) -> Self {
        Self {
            inner,
            reconnect_gate,
        }
    }

    pub(crate) fn send(&self, command: WsCommand) -> Result<(), mpsc::error::SendError<WsCommand>> {
        self.inner.send(command)
    }

    pub(crate) fn request_lag_reconnect(&self) -> bool {
        self.reconnect_gate.request_lag_reconnect(&self.inner)
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(inner: mpsc::UnboundedSender<WsCommand>) -> Self {
        Self::new(inner, WsReconnectGate::default())
    }

    #[cfg(test)]
    pub(crate) fn note_command_dequeued_for_test(&self, command: &WsCommand) {
        self.reconnect_gate.note_dequeued(command);
    }
}

pub(crate) fn get_manager() -> (WsCommandSender, broadcast::Receiver<WsRoutedMessage>) {
    let mgr = WS_MANAGER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = broadcast::channel(10000);
        let reconnect_gate = WsReconnectGate::default();
        let command_sender = WsCommandSender::new(cmd_tx.clone(), reconnect_gate.clone());

        let ping_tx = cmd_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                if ping_tx.send(WsCommand::Ping).is_err() {
                    break;
                }
            }
        });

        tokio::spawn(ws_manager_task_with_reconnect_gate(
            WS_URL.to_string(),
            cmd_rx,
            msg_tx,
            reconnect_gate,
        ));
        WsManager {
            cmd_tx: command_sender,
            msg_rx,
        }
    });
    (mgr.cmd_tx.clone(), mgr.msg_rx.resubscribe())
}

#[cfg(test)]
pub(super) async fn ws_manager_task(
    ws_url: String,
    cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
) {
    ws_manager_task_with_reconnect_gate(ws_url, cmd_rx, msg_tx, WsReconnectGate::default()).await;
}

async fn ws_manager_task_with_reconnect_gate(
    ws_url: String,
    cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
    reconnect_gate: WsReconnectGate,
) {
    ws_manager_task_with_api_probe(
        ws_url,
        cmd_rx,
        msg_tx,
        ApiLatencyProbe::production(),
        reconnect_gate,
    )
    .await;
}

async fn ws_manager_task_with_api_probe(
    ws_url: String,
    cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
    api_probe: ApiLatencyProbe,
    reconnect_gate: WsReconnectGate,
) {
    ws_manager_task_with_options(
        ws_url,
        cmd_rx,
        msg_tx,
        api_probe,
        reconnect_gate,
        Duration::from_secs(WS_CONNECT_TIMEOUT_SECS),
    )
    .await;
}

async fn ws_manager_task_with_options(
    ws_url: String,
    mut cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
    api_probe: ApiLatencyProbe,
    reconnect_gate: WsReconnectGate,
    connect_timeout: Duration,
) {
    let mut active_subs = ActiveWsSubscriptions::default();
    let mut coalescer = CoalescedSender::new(msg_tx);
    use futures::StreamExt as _;
    use futures::future::{Either, select};

    let policy = EXCHANGE_WS_RECONNECT_POLICY;
    let mut reconnect_delay_secs = policy.base_delay_secs;

    'manager: loop {
        if !drain_disconnected_ws_commands(&mut active_subs, &mut cmd_rx, &reconnect_gate) {
            return;
        }
        if reset_reconnect_backoff_if_idle(&active_subs, &mut reconnect_delay_secs, policy)
            && !wait_for_ws_subscription(&mut active_subs, &mut cmd_rx, &reconnect_gate).await
        {
            return;
        }

        let mut connect_fut = Box::pin(connect_with_timeout(
            tokio_tungstenite::connect_async(&ws_url),
            connect_timeout,
        ));
        let connect_result = loop {
            let cmd_fut = Box::pin(cmd_rx.recv());
            match select(connect_fut, cmd_fut).await {
                Either::Left((connect_result, pending_cmd)) => {
                    drop(pending_cmd);
                    break connect_result;
                }
                Either::Right((cmd, pending_connect)) => {
                    let Some(cmd) = cmd else {
                        return;
                    };
                    reconnect_gate.note_dequeued(&cmd);
                    match handle_connecting_ws_command(&mut active_subs, cmd) {
                        ConnectingWsCommandAction::ContinueConnecting => {
                            connect_fut = pending_connect;
                        }
                        ConnectingWsCommandAction::RestartLoop => {
                            drop(pending_connect);
                            continue 'manager;
                        }
                    }
                }
            }
        };

        let ws_stream = match connect_result {
            ConnectAttempt::Finished(Ok((ws, _))) => ws,
            ConnectAttempt::Finished(Err(_)) | ConnectAttempt::TimedOut => {
                if !sleep_with_disconnected_ws_commands(
                    Duration::from_secs(reconnect_delay_secs),
                    &mut active_subs,
                    &mut cmd_rx,
                    &reconnect_gate,
                )
                .await
                {
                    return;
                }
                reconnect_delay_secs = policy.next_delay(reconnect_delay_secs);
                continue;
            }
        };
        telemetry_on_connect();
        let connected_at = Instant::now();

        let (mut write, mut read) = ws_stream.split();
        let mut disconnected = false;
        let mut last_rx_at = Instant::now();
        let mut next_api_probe_at = Instant::now();

        if !drain_disconnected_ws_commands(&mut active_subs, &mut cmd_rx, &reconnect_gate) {
            return;
        }
        if active_subs.is_empty() {
            disconnected = true;
        }

        for payload in active_subs.payloads() {
            if !send_ws_text_with_timeout(&mut write, payload.to_string()).await {
                disconnected = true;
                break;
            }
        }

        while !disconnected {
            let now = Instant::now();
            if now >= next_api_probe_at {
                api_probe.spawn();
                next_api_probe_at = now + API_LATENCY_PROBE_INTERVAL;
            }

            let cmd_fut = Box::pin(cmd_rx.recv());
            let read_fut = Box::pin(read.next());
            let stale_in = stale_read_remaining(last_rx_at.elapsed());
            let api_probe_in = next_api_probe_at.saturating_duration_since(Instant::now());
            let timeout_in = read_loop_timeout(stale_in, coalescer.next_due()).min(api_probe_in);

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
                    reconnect_gate.note_dequeued(&cmd);
                    let action = handle_ws_command(&mut active_subs, cmd);
                    if action.mark_ping_start {
                        telemetry_mark_ws_ping_start();
                    }
                    if let Some(payload) = action.outbound_payload
                        && !send_ws_text_with_timeout(&mut write, payload.to_string()).await
                        && action.disconnect_on_send_error
                    {
                        disconnected = true;
                    }
                    if action.disconnect_after_handling {
                        disconnected = true;
                    }
                    if active_subs.is_empty() {
                        disconnected = true;
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
        if !sleep_with_disconnected_ws_commands(
            Duration::from_secs(delay_secs),
            &mut active_subs,
            &mut cmd_rx,
            &reconnect_gate,
        )
        .await
        {
            return;
        }
        reconnect_delay_secs = next_delay_secs;
    }
}

#[derive(Clone)]
enum ApiLatencyProbe {
    #[cfg(not(test))]
    Network,
    #[cfg(test)]
    Disabled,
    #[cfg(test)]
    Notify(mpsc::UnboundedSender<()>),
}

impl ApiLatencyProbe {
    fn production() -> Self {
        #[cfg(not(test))]
        {
            Self::Network
        }
        #[cfg(test)]
        {
            Self::Disabled
        }
    }

    fn spawn(&self) {
        match self {
            #[cfg(not(test))]
            Self::Network => {
                tokio::spawn(update_api_latency_once());
            }
            #[cfg(test)]
            Self::Disabled => {}
            #[cfg(test)]
            Self::Notify(probe_tx) => {
                let _ = probe_tx.send(());
            }
        }
    }
}

#[cfg(test)]
pub(super) async fn ws_manager_task_with_api_probe_notifier(
    ws_url: String,
    cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
    probe_tx: mpsc::UnboundedSender<()>,
) {
    ws_manager_task_with_api_probe(
        ws_url,
        cmd_rx,
        msg_tx,
        ApiLatencyProbe::Notify(probe_tx),
        WsReconnectGate::default(),
    )
    .await;
}

#[cfg(test)]
pub(super) async fn ws_manager_task_with_connect_timeout_for_test(
    ws_url: String,
    cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    msg_tx: broadcast::Sender<WsRoutedMessage>,
    connect_timeout: Duration,
) {
    ws_manager_task_with_options(
        ws_url,
        cmd_rx,
        msg_tx,
        ApiLatencyProbe::Disabled,
        WsReconnectGate::default(),
        connect_timeout,
    )
    .await;
}

#[cfg(not(test))]
async fn update_api_latency_once() {
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
}

async fn send_ws_text_with_timeout<W>(write: &mut W, text: String) -> bool
where
    W: Sink<WsMsg> + Unpin,
{
    telemetry_add_tx(text.len() as u64);
    let mut send = std::pin::pin!(write.send(WsMsg::Text(text.into())));
    let first_poll = futures::future::poll_fn(|cx| {
        std::task::Poll::Ready(std::future::Future::poll(send.as_mut(), cx))
    })
    .await;
    if let std::task::Poll::Ready(result) = first_poll {
        return result.is_ok();
    }
    tokio::time::timeout(WS_WRITE_TIMEOUT, send)
        .await
        .is_ok_and(|result| result.is_ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectingWsCommandAction {
    ContinueConnecting,
    RestartLoop,
}

fn reset_reconnect_backoff_if_idle(
    active_subs: &ActiveWsSubscriptions,
    reconnect_delay_secs: &mut u64,
    policy: ReconnectPolicy,
) -> bool {
    if active_subs.is_empty() {
        *reconnect_delay_secs = policy.base_delay_secs;
        true
    } else {
        false
    }
}

async fn wait_for_ws_subscription(
    active_subs: &mut ActiveWsSubscriptions,
    cmd_rx: &mut mpsc::UnboundedReceiver<WsCommand>,
    reconnect_gate: &WsReconnectGate,
) -> bool {
    while active_subs.is_empty() {
        let Some(command) = cmd_rx.recv().await else {
            return false;
        };
        reconnect_gate.note_dequeued(&command);
        handle_disconnected_ws_command(active_subs, command);
    }
    true
}

fn handle_connecting_ws_command(
    active_subs: &mut ActiveWsSubscriptions,
    command: WsCommand,
) -> ConnectingWsCommandAction {
    match command {
        WsCommand::Subscribe { topic, payload } => {
            active_subs.subscribe(topic, payload);
            ConnectingWsCommandAction::ContinueConnecting
        }
        WsCommand::Unsubscribe { topic, payload } => {
            active_subs.unsubscribe(topic, payload);
            if active_subs.is_empty() {
                ConnectingWsCommandAction::RestartLoop
            } else {
                ConnectingWsCommandAction::ContinueConnecting
            }
        }
        WsCommand::Ping => ConnectingWsCommandAction::ContinueConnecting,
        WsCommand::Reconnect => ConnectingWsCommandAction::RestartLoop,
    }
}

fn handle_disconnected_ws_command(active_subs: &mut ActiveWsSubscriptions, command: WsCommand) {
    match command {
        WsCommand::Subscribe { topic, payload } => {
            active_subs.subscribe(topic, payload);
        }
        WsCommand::Unsubscribe { topic, payload } => {
            active_subs.unsubscribe(topic, payload);
        }
        WsCommand::Ping | WsCommand::Reconnect => {}
    }
}

fn drain_disconnected_ws_commands(
    active_subs: &mut ActiveWsSubscriptions,
    cmd_rx: &mut mpsc::UnboundedReceiver<WsCommand>,
    reconnect_gate: &WsReconnectGate,
) -> bool {
    loop {
        match cmd_rx.try_recv() {
            Ok(command) => {
                reconnect_gate.note_dequeued(&command);
                handle_disconnected_ws_command(active_subs, command);
            }
            Err(mpsc::error::TryRecvError::Empty) => return true,
            Err(mpsc::error::TryRecvError::Disconnected) => return false,
        }
    }
}

async fn sleep_with_disconnected_ws_commands(
    delay: Duration,
    active_subs: &mut ActiveWsSubscriptions,
    cmd_rx: &mut mpsc::UnboundedReceiver<WsCommand>,
    reconnect_gate: &WsReconnectGate,
) -> bool {
    if !drain_disconnected_ws_commands(active_subs, cmd_rx, reconnect_gate) {
        return false;
    }
    if active_subs.is_empty() {
        return true;
    }

    let sleep = tokio::time::sleep(delay);
    tokio::pin!(sleep);

    loop {
        tokio::select! {
            () = &mut sleep => {
                return drain_disconnected_ws_commands(active_subs, cmd_rx, reconnect_gate);
            }
            command = cmd_rx.recv() => {
                let Some(command) = command else {
                    return false;
                };
                reconnect_gate.note_dequeued(&command);
                handle_disconnected_ws_command(active_subs, command);
                if active_subs.is_empty() {
                    return true;
                }
            }
        }
    }
}

pub(crate) struct SubscriptionGuard {
    pub(crate) cmd_tx: WsCommandSender,
    pub(crate) subscriptions: Vec<(String, Value)>,
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        for (topic, payload) in &self.subscriptions {
            let _ = self.cmd_tx.send(WsCommand::Unsubscribe {
                topic: topic.clone(),
                payload: payload.clone(),
            });
        }
    }
}
