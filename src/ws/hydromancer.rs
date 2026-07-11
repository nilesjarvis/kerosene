mod liquidations;
mod manager;
mod market_streams;
mod parsing;
mod recent;
mod tracked_trades;

use super::WsStream;
use super::telemetry::{now_ms, telemetry_update_hydromancer_api_latency};
use crate::api::CLIENT;
use crate::hydromancer_api::HYDROMANCER_API_URL;
use crate::message::Message;
use futures::SinkExt as _;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::time::Duration;
#[cfg(test)]
use tokio::sync::mpsc;
use zeroize::Zeroizing;

pub use liquidations::ws_hydromancer_liquidations;
#[cfg(test)]
pub(crate) use manager::hydromancer_manager_reconnect_sent_for_test;
pub use manager::{evict_hydromancer_manager, reconnect_hydromancer};
pub use market_streams::{
    ws_hydromancer_asset_ctx_stream_keyed, ws_hydromancer_asset_ctx_stream_symbol,
    ws_hydromancer_book_stream_keyed_events, ws_hydromancer_candle_stream_keyed,
    ws_hydromancer_spaghetti_candle_stream,
};
pub use tracked_trades::ws_hydromancer_tracked_trades;

const HYDROMANCER_RECONNECT_DELAY_SECS: u64 = 2;

#[derive(Clone, PartialEq, Eq)]
pub struct HydromancerStreamKey {
    api_key: Zeroizing<String>,
    generation: u64,
}

impl HydromancerStreamKey {
    pub(crate) fn new(api_key: impl AsRef<str>, generation: u64) -> Self {
        Self {
            api_key: Zeroizing::new(api_key.as_ref().trim().to_string()),
            generation,
        }
    }

    pub(crate) fn from_zeroizing(api_key: Zeroizing<String>, generation: u64) -> Self {
        Self {
            api_key: Zeroizing::new(api_key.trim().to_string()),
            generation,
        }
    }

    pub(crate) fn api_key_for_task(&self) -> Zeroizing<String> {
        self.api_key.clone()
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn manager_id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.generation.hash(&mut hasher);
        self.api_key.as_str().hash(&mut hasher);
        hasher.finish()
    }
}

impl fmt::Debug for HydromancerStreamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HydromancerStreamKey")
            .field("api_key", &"<redacted>")
            .field("generation", &self.generation)
            .finish()
    }
}

impl Hash for HydromancerStreamKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.generation.hash(state);
        self.api_key.as_str().hash(state);
    }
}

fn request_hydromancer_reconnect_after_lag(cmd_tx: &manager::HydromancerCommandSender) -> bool {
    cmd_tx.request_lag_reconnect()
}

async fn emit_hydromancer_lag_after_reconnect<T, Emit, Fut>(
    cmd_tx: &manager::HydromancerCommandSender,
    event: T,
    emit: Emit,
    pause: Duration,
) -> bool
where
    Emit: FnOnce(T) -> Fut,
    Fut: Future<Output = bool>,
{
    if !request_hydromancer_reconnect_after_lag(cmd_tx) {
        return false;
    }
    if !emit(event).await {
        return false;
    }
    if !pause.is_zero() {
        tokio::time::sleep(pause).await;
    }
    true
}

// ---------------------------------------------------------------------------
// Hydromancer REST Latency Probe
// ---------------------------------------------------------------------------

#[cfg(not(test))]
const HYDROMANCER_LATENCY_PROBE_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(test)]
const HYDROMANCER_LATENCY_PROBE_INTERVAL: Duration = Duration::from_millis(50);

/// Performs a lightweight `exchangeStatus` request against the Hydromancer REST
/// API and records the round-trip latency into ws telemetry. Scheduled by the
/// subscription layer while a Hydromancer API key is configured.
fn hydromancer_api_latency_probe_payload() -> serde_json::Value {
    serde_json::json!({ "type": "exchangeStatus" })
}

async fn update_hydromancer_api_latency_once(api_key: Zeroizing<String>) {
    let start_time = now_ms();
    let payload = hydromancer_api_latency_probe_payload();
    if let Ok(resp) = CLIENT
        .clone()
        .post(HYDROMANCER_API_URL)
        .bearer_auth(api_key.trim())
        .json(&payload)
        .send()
        .await
        && resp.status().is_success()
    {
        let latency = now_ms().saturating_sub(start_time);
        telemetry_update_hydromancer_api_latency(latency);
    }
}

/// Subscription recipe: periodically probes Hydromancer REST latency while a
/// Hydromancer API key is configured. Yields `Message::NoOp` after each probe;
/// the status bar ticks every second and re-reads telemetry, so the updated
/// latency surfaces without a dedicated message.
pub fn ws_hydromancer_api_latency_probe(stream_key: &HydromancerStreamKey) -> WsStream<Message> {
    let api_key = stream_key.api_key_for_task();
    Box::pin(iced::stream::channel(1, async move |mut output| {
        loop {
            update_hydromancer_api_latency_once(api_key.clone()).await;
            if output.send(Message::NoOp).await.is_err() {
                return;
            }
            tokio::time::sleep(HYDROMANCER_LATENCY_PROBE_INTERVAL).await;
        }
    }))
}

#[derive(Clone)]
pub struct LiquidationEvent {
    pub coin: String,
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
    pub time_ms: u64,
    pub method: String,
    pub liquidated_user: String,
    pub tx_index: u64,
}

impl fmt::Debug for LiquidationEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiquidationEvent")
            .field("coin", &self.coin)
            .field("price", &self.price)
            .field("size", &self.size)
            .field("is_buy", &self.is_buy)
            .field("time_ms", &self.time_ms)
            .field("method", &self.method)
            .field("liquidated_user", &"<redacted>")
            .field("tx_index", &self.tx_index)
            .finish()
    }
}

#[derive(Clone)]
pub struct TrackedTradeEvent {
    pub address: String,
    pub coin: String,
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
    pub time_ms: u64,
    pub dir: String,
    pub start_position: Option<f64>,
    pub closed_pnl: f64,
    pub fee: f64,
    pub fee_token: String,
    pub tid: Option<u64>,
    pub hash: String,
    pub oid: Option<u64>,
    pub tx_index: u64,
}

impl fmt::Debug for TrackedTradeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrackedTradeEvent")
            .field("address", &"<redacted>")
            .field("coin", &"<redacted>")
            .field("price", &"<redacted>")
            .field("size", &"<redacted>")
            .field("is_buy", &self.is_buy)
            .field("time_ms", &"<redacted>")
            .field("dir", &"<redacted>")
            .field("has_start_position", &self.start_position.is_some())
            .field("closed_pnl", &"<redacted>")
            .field("fee", &"<redacted>")
            .field("fee_token", &"<redacted>")
            .field("has_tid", &self.tid.is_some())
            .field("hash", &"<redacted>")
            .field("has_oid", &self.oid.is_some())
            .field("tx_index", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
pub enum HydromancerWsMessage {
    Connecting,
    Resuming,
    Connected,
    Reconnected,
    Heartbeat,
    Reconnecting {
        error: String,
        retry_delay_secs: u64,
    },
    Disconnected(String),
    Lagged {
        skipped: u64,
    },
    Event(LiquidationEvent),
    TrackedTrade(TrackedTradeEvent),
}

impl fmt::Debug for HydromancerWsMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connecting => f.write_str("Connecting"),
            Self::Resuming => f.write_str("Resuming"),
            Self::Connected => f.write_str("Connected"),
            Self::Reconnected => f.write_str("Reconnected"),
            Self::Heartbeat => f.write_str("Heartbeat"),
            Self::Reconnecting {
                retry_delay_secs, ..
            } => f
                .debug_struct("Reconnecting")
                .field("error", &"<redacted>")
                .field("retry_delay_secs", retry_delay_secs)
                .finish(),
            Self::Disconnected(_) => f.debug_tuple("Disconnected").field(&"<redacted>").finish(),
            Self::Lagged { skipped } => f.debug_struct("Lagged").field("skipped", skipped).finish(),
            Self::Event(event) => f.debug_tuple("Event").field(event).finish(),
            Self::TrackedTrade(event) => f.debug_tuple("TrackedTrade").field(event).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_value(value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn hydromancer_api_latency_probe_uses_exchange_status_health_check() {
        assert_eq!(
            hydromancer_api_latency_probe_payload(),
            serde_json::json!({ "type": "exchangeStatus" })
        );
    }

    #[test]
    fn lag_reconnect_helper_sends_reconnect_command() {
        let (raw_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let cmd_tx = manager::HydromancerCommandSender::new_for_test(raw_cmd_tx);

        assert!(request_hydromancer_reconnect_after_lag(&cmd_tx));
        assert!(matches!(
            cmd_rx.try_recv().unwrap(),
            manager::HydromancerCommand::Reconnect
        ));
    }

    #[test]
    fn lag_reconnect_helper_coalesces_until_reconnect_dequeued() {
        let (raw_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let cmd_tx = manager::HydromancerCommandSender::new_for_test(raw_cmd_tx);

        assert!(request_hydromancer_reconnect_after_lag(&cmd_tx));
        assert!(request_hydromancer_reconnect_after_lag(&cmd_tx));
        let command = cmd_rx.try_recv().expect("first reconnect command");
        assert!(matches!(command, manager::HydromancerCommand::Reconnect));
        assert!(cmd_rx.try_recv().is_err());

        cmd_tx.note_command_dequeued_for_test(&command);
        assert!(request_hydromancer_reconnect_after_lag(&cmd_tx));
        assert!(matches!(
            cmd_rx.try_recv().unwrap(),
            manager::HydromancerCommand::Reconnect
        ));
    }

    #[tokio::test]
    async fn lag_emit_requests_reconnect_before_downstream_send_failure() {
        let (raw_cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let cmd_tx = manager::HydromancerCommandSender::new_for_test(raw_cmd_tx);

        let emitted = emit_hydromancer_lag_after_reconnect(
            &cmd_tx,
            HydromancerWsMessage::Lagged { skipped: 7 },
            |_event| async { false },
            Duration::ZERO,
        )
        .await;

        assert!(!emitted);
        assert!(matches!(
            cmd_rx.try_recv().unwrap(),
            manager::HydromancerCommand::Reconnect
        ));
    }

    #[test]
    fn lagged_message_has_no_payload_content() {
        let HydromancerWsMessage::Lagged { skipped } =
            (HydromancerWsMessage::Lagged { skipped: 42 })
        else {
            panic!("expected lagged message");
        };

        assert_eq!(skipped, 42);
    }

    #[test]
    fn hydromancer_control_message_debug_redacts_error_strings() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        let reconnecting = HydromancerWsMessage::Reconnecting {
            error: format!("stream failed for {ADDRESS} with token payload-secret"),
            retry_delay_secs: 2,
        };
        let disconnected = HydromancerWsMessage::Disconnected(format!("disconnect for {ADDRESS}"));

        let rendered = format!("{reconnecting:?} {disconnected:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("retry_delay_secs: 2"));
        assert!(!rendered.contains(ADDRESS));
        assert!(!rendered.contains("payload-secret"));
    }

    #[test]
    fn hydromancer_stream_key_redacts_debug_and_hashes_key_and_generation() {
        let first = HydromancerStreamKey::new("hydro-secret-a", 7);
        let rotated_same_generation = HydromancerStreamKey::new("hydro-secret-b", 7);
        let next_generation = HydromancerStreamKey::new("hydro-secret-b", 8);

        let debug = format!("{first:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("hydro-secret-a"));

        assert_ne!(hash_value(&first), hash_value(&rotated_same_generation));
        assert_ne!(hash_value(&first), hash_value(&next_generation));
    }

    #[test]
    fn hydromancer_tracked_trade_debug_redacts_account_trade_values() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const PRICE: f64 = 98_765.432_1;
        const SIZE: f64 = 12_345.678_9;

        let event = TrackedTradeEvent {
            address: ADDRESS.to_string(),
            coin: "private-tracked-coin-sentinel".to_string(),
            price: PRICE,
            size: SIZE,
            is_buy: true,
            time_ms: 9_876_543_210,
            dir: "private-tracked-direction-sentinel".to_string(),
            start_position: Some(-45_678.912_3),
            closed_pnl: -33_333.233_4,
            fee: 22_222.122_3,
            fee_token: "private-tracked-fee-token-sentinel".to_string(),
            tid: Some(98_765_432),
            hash: "private-tracked-hash-sentinel".to_string(),
            oid: Some(12_345_678),
            tx_index: 87_654_321,
        };
        let message = HydromancerWsMessage::TrackedTrade(event.clone());

        let debug = format!("{message:?}");

        assert!(debug.contains("<redacted>"));
        for sensitive in [
            ADDRESS.to_string(),
            "private-tracked-coin-sentinel".to_string(),
            format!("{PRICE:?}"),
            format!("{SIZE:?}"),
            "9876543210".to_string(),
            "private-tracked-direction-sentinel".to_string(),
            "private-tracked-fee-token-sentinel".to_string(),
            "private-tracked-hash-sentinel".to_string(),
            "98765432".to_string(),
            "12345678".to_string(),
            "87654321".to_string(),
        ] {
            assert!(!debug.contains(&sensitive), "{debug}");
        }
        assert_eq!(event.address, ADDRESS);
        assert_eq!(event.coin, "private-tracked-coin-sentinel");
        assert_eq!(event.price.to_bits(), PRICE.to_bits());
        assert_eq!(event.size.to_bits(), SIZE.to_bits());
        assert_eq!(event.oid, Some(12_345_678));
    }

    #[test]
    fn hydromancer_liquidation_debug_redacts_user_address() {
        const ADDRESS: &str = "0xdef0000000000000000000000000000000000000";

        let message = HydromancerWsMessage::Event(LiquidationEvent {
            coin: "HYPE".to_string(),
            price: 10.0,
            size: 1.0,
            is_buy: false,
            time_ms: 100,
            method: "market".to_string(),
            liquidated_user: ADDRESS.to_string(),
            tx_index: 7,
        });

        let debug = format!("{message:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains(ADDRESS));
    }
}
