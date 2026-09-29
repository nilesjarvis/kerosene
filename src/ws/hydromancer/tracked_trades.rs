use super::fill_stream::{FillFeed, forward_fill_stream};
use super::manager::get_hydromancer_manager;
use super::{HydromancerStreamKey, HydromancerWsMessage};
use crate::ws::WsStream;

use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Tracked Trade Stream
// ---------------------------------------------------------------------------

fn tracked_trade_subscription(addresses: Vec<String>) -> Option<(String, Value)> {
    if addresses.is_empty() {
        return None;
    }

    Some((
        format!("userFills:{}", addresses.join(",")),
        serde_json::json!({
            "type": "subscribe",
            "subscription": {
                "type": "userFills",
                "addresses": addresses,
                "aggregateByTime": true
            }
        }),
    ))
}

pub fn ws_hydromancer_tracked_trades(
    stream_key: &(HydromancerStreamKey, u64, Vec<String>),
) -> WsStream<HydromancerWsMessage> {
    let manager_key = stream_key.0.clone();
    let addresses = stream_key.2.clone();

    Box::pin(iced::stream::channel(10000, async move |output| {
        let Some((topic, payload)) = tracked_trade_subscription(addresses) else {
            return;
        };

        let manager = get_hydromancer_manager(manager_key);
        forward_fill_stream(FillFeed::TrackedTrades, (topic, payload), manager, output).await;
    }))
}
