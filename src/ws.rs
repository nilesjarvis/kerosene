mod connect;
mod hydromancer;
mod l2_book;
mod manager;
mod market_streams;
mod telemetry;
mod user_streams;

pub(crate) use l2_book::{
    L2BookSigfigs, l2_book_payload_matches_sigfigs, l2_book_sigfigs_from_value,
};
pub use manager::WsCommand;
pub(crate) use manager::{SubscriptionGuard, get_manager};
pub(crate) use telemetry::now_ms;
pub use telemetry::telemetry_snapshot;
pub(crate) use telemetry::{
    telemetry_add_hydromancer_rx, telemetry_add_hydromancer_tx, telemetry_on_hydromancer_connect,
    telemetry_on_hydromancer_disconnect,
};

#[cfg(test)]
pub(crate) use hydromancer::hydromancer_manager_reconnect_sent_for_test;
pub use hydromancer::{
    HydromancerStreamKey, HydromancerWsMessage, LiquidationEvent, TrackedTradeEvent,
    evict_hydromancer_manager, reconnect_hydromancer, ws_hydromancer_asset_ctx_stream_keyed,
    ws_hydromancer_asset_ctx_stream_symbol, ws_hydromancer_book_stream_keyed_events,
    ws_hydromancer_candle_stream_keyed, ws_hydromancer_liquidations,
    ws_hydromancer_spaghetti_candle_stream, ws_hydromancer_tracked_trades,
};
pub use market_streams::{
    KeyedAssetContextStreamEvent, KeyedBookStreamEvent, KeyedCandleStreamEvent,
    SpaghettiCandleStreamEvent, SymbolAssetContextStreamEvent, ws_asset_ctx_stream_keyed,
    ws_asset_ctx_stream_symbol, ws_book_stream_keyed_events, ws_candle_stream_keyed,
    ws_spaghetti_candle_stream,
};
pub use user_streams::{WsUserData, WsUserDataStreamParams, ws_user_data_stream};

// ---------------------------------------------------------------------------
// WebSocket streams
// ---------------------------------------------------------------------------

pub const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
pub type WsStream<T> = std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>;

pub(crate) fn broadcast_receiver_closed(error: &tokio::sync::broadcast::error::RecvError) -> bool {
    matches!(error, tokio::sync::broadcast::error::RecvError::Closed)
}

#[cfg(test)]
mod tests {
    use super::broadcast_receiver_closed;
    use tokio::sync::broadcast::error::RecvError;

    #[test]
    fn broadcast_receiver_closed_only_matches_closed_error() {
        assert!(broadcast_receiver_closed(&RecvError::Closed));
        assert!(!broadcast_receiver_closed(&RecvError::Lagged(3)));
    }
}
