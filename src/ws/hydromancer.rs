mod liquidations;
mod manager;
mod market_streams;
mod parsing;
mod recent;
mod tracked_trades;

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::time::Duration;
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

fn request_hydromancer_reconnect_after_lag(
    cmd_tx: &mpsc::UnboundedSender<manager::HydromancerCommand>,
) -> bool {
    cmd_tx.send(manager::HydromancerCommand::Reconnect).is_ok()
}

async fn emit_hydromancer_lag_after_reconnect<T, Emit, Fut>(
    cmd_tx: &mpsc::UnboundedSender<manager::HydromancerCommand>,
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
    fn lag_reconnect_helper_sends_reconnect_command() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        assert!(request_hydromancer_reconnect_after_lag(&cmd_tx));
        assert!(matches!(
            cmd_rx.try_recv().unwrap(),
            manager::HydromancerCommand::Reconnect
        ));
    }

    #[tokio::test]
    async fn lag_emit_requests_reconnect_before_downstream_send_failure() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

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
}
