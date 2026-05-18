mod hydromancer;
mod manager;
mod market_streams;
mod telemetry;
mod user_streams;

pub use manager::WsCommand;
pub(crate) use manager::{SubscriptionGuard, get_manager};
pub use telemetry::{now_ms, telemetry_snapshot};
pub(crate) use telemetry::{
    telemetry_add_rx, telemetry_add_tx, telemetry_on_connect, telemetry_on_disconnect,
};

pub use hydromancer::{
    HydromancerWsMessage, LiquidationEvent, TrackedTradeEvent, evict_hydromancer_manager,
    reconnect_hydromancer, ws_hydromancer_liquidations, ws_hydromancer_tracked_trades,
};
pub use market_streams::{
    ws_asset_ctx_stream_keyed, ws_asset_ctx_stream_symbol, ws_book_stream_keyed,
    ws_candle_stream_keyed, ws_spaghetti_candle_stream,
};
pub use user_streams::{WsUserData, WsUserDataStreamParams, ws_user_data_stream};

// ---------------------------------------------------------------------------
// WebSocket streams
// ---------------------------------------------------------------------------

pub const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
pub type WsStream<T> = std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>;
