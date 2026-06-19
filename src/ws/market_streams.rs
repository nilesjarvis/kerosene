mod asset_context;
mod books;
mod candles;

use crate::account::AssetContext;
use crate::api::Candle;
use crate::spaghetti;
use crate::timeframe::Timeframe;
use crate::ws::WsCommand;
use std::{future::Future, time::Duration};
use tokio::sync::mpsc;

pub use asset_context::ws_asset_ctx_stream_keyed;
pub use asset_context::ws_asset_ctx_stream_symbol;
pub use books::ws_book_stream_keyed_events;
pub use candles::{ws_candle_stream_keyed, ws_spaghetti_candle_stream};

#[derive(Debug, Clone, PartialEq)]
pub enum WsStreamEvent<T> {
    Item(T),
    Lagged { skipped: u64 },
}

#[derive(Debug, Clone)]
pub enum KeyedBookStreamEvent {
    Item(
        u64,
        String,
        (Option<u8>, Option<u8>),
        Option<u64>,
        crate::api::OrderBook,
    ),
    Lagged {
        id: u64,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        hydromancer_key_generation: Option<u64>,
        skipped: u64,
    },
}

#[derive(Debug, Clone)]
pub enum KeyedAssetContextStreamEvent {
    Item(u64, String, Option<u64>, Box<AssetContext>),
    Lagged {
        id: u64,
        symbol: String,
        hydromancer_key_generation: Option<u64>,
        skipped: u64,
    },
}

#[derive(Debug, Clone)]
pub enum SymbolAssetContextStreamEvent {
    Item(String, Option<u64>, Box<AssetContext>),
    Lagged {
        symbol: String,
        hydromancer_key_generation: Option<u64>,
        skipped: u64,
    },
}

#[derive(Debug, Clone)]
pub enum KeyedCandleStreamEvent {
    Item(u64, String, String, Option<u64>, Candle),
    Lagged {
        id: u64,
        symbol: String,
        interval: String,
        hydromancer_key_generation: Option<u64>,
        skipped: u64,
    },
}

#[derive(Debug, Clone)]
pub enum SpaghettiCandleStreamEvent {
    Item {
        id: u64,
        symbol: String,
        timeframe: Timeframe,
        hydromancer_key_generation: Option<u64>,
        session: Option<spaghetti::Session>,
        session_granularity: Option<Timeframe>,
        candle: Candle,
    },
    Lagged {
        id: u64,
        symbol: String,
        timeframe: Timeframe,
        hydromancer_key_generation: Option<u64>,
        session: Option<spaghetti::Session>,
        session_granularity: Option<Timeframe>,
        skipped: u64,
    },
}

const WS_LAG_RECONNECT_PAUSE_SECS: u64 = 2;

fn request_ws_reconnect_after_lag(cmd_tx: &mpsc::UnboundedSender<WsCommand>) -> bool {
    cmd_tx.send(WsCommand::Reconnect).is_ok()
}

async fn emit_lag_after_reconnect<T, Emit, Fut>(
    cmd_tx: &mpsc::UnboundedSender<WsCommand>,
    event: T,
    emit: Emit,
    pause: Duration,
) -> bool
where
    Emit: FnOnce(T) -> Fut,
    Fut: Future<Output = bool>,
{
    if !request_ws_reconnect_after_lag(cmd_tx) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lag_reconnect_helper_sends_reconnect_command() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        assert!(request_ws_reconnect_after_lag(&cmd_tx));
        assert!(matches!(cmd_rx.try_recv().unwrap(), WsCommand::Reconnect));
    }

    #[tokio::test]
    async fn lag_emit_requests_reconnect_before_downstream_send_failure() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        let emitted = emit_lag_after_reconnect(
            &cmd_tx,
            WsStreamEvent::<()>::Lagged { skipped: 7 },
            |_event| async { false },
            Duration::ZERO,
        )
        .await;

        assert!(!emitted);
        assert!(matches!(cmd_rx.try_recv().unwrap(), WsCommand::Reconnect));
    }
}
