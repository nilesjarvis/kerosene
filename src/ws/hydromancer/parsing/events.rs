use super::super::{LiquidationEvent, TrackedTradeEvent};
use super::fills::{fill_address_and_details, hydromancer_str_f64, hydromancer_u64};
use serde_json::Value;

pub(in crate::ws::hydromancer) fn parse_liquidation_event(
    fill_tuple: &Value,
) -> Option<LiquidationEvent> {
    let (_address, details) = fill_address_and_details(fill_tuple)?;
    let liquidation = details.get("liquidation")?;

    Some(LiquidationEvent {
        coin: details
            .get("coin")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        price: hydromancer_str_f64(details, "px")?,
        size: hydromancer_str_f64(details, "sz")?,
        is_buy: details.get("side").and_then(|v| v.as_str()) == Some("B"),
        time_ms: hydromancer_u64(details, "time"),
        method: liquidation
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        liquidated_user: liquidation
            .get("liquidatedUser")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        tx_index: hydromancer_u64(details, "txIndex"),
    })
}

pub(in crate::ws::hydromancer) fn parse_tracked_trade_event(
    fill_tuple: &Value,
) -> Option<TrackedTradeEvent> {
    let (address, details) = fill_address_and_details(fill_tuple)?;

    Some(TrackedTradeEvent {
        address,
        coin: details
            .get("coin")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        price: hydromancer_str_f64(details, "px")?,
        size: hydromancer_str_f64(details, "sz")?,
        is_buy: details.get("side").and_then(|v| v.as_str()) == Some("B"),
        time_ms: hydromancer_u64(details, "time"),
        dir: details
            .get("dir")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        start_position: details
            .get("startPosition")
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite()),
        closed_pnl: hydromancer_str_f64(details, "closedPnl")?,
        fee: hydromancer_str_f64(details, "fee")?,
        fee_token: details
            .get("feeToken")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        tid: details.get("tid").and_then(|v| v.as_u64()),
        hash: details
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        oid: details.get("oid").and_then(|v| v.as_u64()),
        tx_index: hydromancer_u64(details, "txIndex"),
    })
}

pub(in crate::ws::hydromancer) fn liquidation_dedupe_key(event: &LiquidationEvent) -> String {
    format!(
        "liq:{}:{}:{}:{:.8}:{:.8}:{}",
        event.time_ms,
        event.tx_index,
        event.coin,
        event.price,
        event.size,
        event.liquidated_user.to_ascii_lowercase()
    )
}

pub(in crate::ws::hydromancer) fn tracked_trade_dedupe_key(event: &TrackedTradeEvent) -> String {
    format!(
        "trade:{}:{}:{}:{}:{}:{}",
        event.address.to_ascii_lowercase(),
        event.time_ms,
        event.tx_index,
        event.tid.map(|v| v.to_string()).unwrap_or_default(),
        event.hash,
        event.oid.map(|v| v.to_string()).unwrap_or_default()
    )
}
