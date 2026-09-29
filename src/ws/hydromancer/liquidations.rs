use super::fill_stream::{FillFeed, forward_fill_stream};
use super::manager::get_hydromancer_manager;
use super::{HydromancerStreamKey, HydromancerWsMessage};
use crate::ws::WsStream;

use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Liquidation Stream
// ---------------------------------------------------------------------------

fn liquidation_subscription() -> (String, Value) {
    (
        "liquidationFills".to_string(),
        serde_json::json!({
            "type": "subscribe",
            "subscription": {
                "type": "liquidationFills"
            }
        }),
    )
}

pub fn ws_hydromancer_liquidations(
    stream_key: &(HydromancerStreamKey, u64),
) -> WsStream<HydromancerWsMessage> {
    let manager_key = stream_key.0.clone();
    Box::pin(iced::stream::channel(10000, async move |output| {
        let manager = get_hydromancer_manager(manager_key);
        let (topic, payload) = liquidation_subscription();
        forward_fill_stream(FillFeed::Liquidations, (topic, payload), manager, output).await;
    }))
}
