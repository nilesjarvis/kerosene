mod asset_context;
mod books;
mod candles;
mod payloads;

use super::HydromancerWsMessage;
use super::capacity::{MARKET_FALLBACK_MESSAGE, is_capacity_error};

fn hydromancer_market_control_message(
    msg_type: &str,
    data: &serde_json::Value,
) -> Option<HydromancerWsMessage> {
    if msg_type == MARKET_FALLBACK_MESSAGE {
        Some(HydromancerWsMessage::Disconnected(
            "Hydromancer subscription limit reached".to_string(),
        ))
    } else {
        super::parsing::hydromancer_control_message(msg_type, data)
    }
}

fn hydromancer_candle_control_message(
    interval: &str,
    msg_type: &str,
    data: &serde_json::Value,
) -> Option<HydromancerWsMessage> {
    if interval == "1s" {
        // Capacity notices only retire replaceable streams, not this one.
        super::parsing::hydromancer_control_message(msg_type, data)
    } else {
        hydromancer_market_control_message(msg_type, data)
    }
}

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
    is_capacity_error(error)
        || lower.contains("authentication failed")
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
