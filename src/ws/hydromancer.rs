mod liquidations;
mod manager;
mod parsing;
mod recent;
mod tracked_trades;

pub use liquidations::ws_hydromancer_liquidations;
pub use manager::{evict_hydromancer_manager, reconnect_hydromancer};
pub use tracked_trades::ws_hydromancer_tracked_trades;

const HYDROMANCER_RECONNECT_DELAY_SECS: u64 = 2;

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
    Event(LiquidationEvent),
    TrackedTrade(TrackedTradeEvent),
}
