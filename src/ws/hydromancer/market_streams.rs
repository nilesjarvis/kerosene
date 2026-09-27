mod asset_context;
mod books;
mod candles;
mod payloads;

use super::HydromancerWsMessage;

pub use asset_context::{
    ws_hydromancer_asset_ctx_stream_keyed, ws_hydromancer_asset_ctx_stream_symbol,
};
pub use books::ws_hydromancer_book_stream_keyed_events;
pub use candles::{ws_hydromancer_candle_stream_keyed, ws_hydromancer_spaghetti_candle_stream};

fn hydromancer_market_control_should_fallback(control: &HydromancerWsMessage) -> bool {
    match control {
        HydromancerWsMessage::Reconnecting { error, .. } => {
            hydromancer_market_disconnect_should_fallback(error)
        }
        HydromancerWsMessage::Disconnected(error) => {
            hydromancer_market_disconnect_should_fallback(error)
        }
        _ => false,
    }
}

fn hydromancer_market_disconnect_should_fallback(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("authentication failed")
        || lower.contains("check the api key")
        || lower.contains("unauthorized")
        || lower.contains("unauthenticated")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid token")
        || lower.contains("http 401")
        || lower.contains("http 403")
}

#[cfg(test)]
mod tests;
