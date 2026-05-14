use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// User Stream Subscriptions
// ---------------------------------------------------------------------------

pub(super) fn build_user_stream_subscriptions(
    address: Option<&str>,
    dexes: &[String],
) -> Vec<(String, Value)> {
    let mut subscriptions = vec![(
        "allMids".to_string(),
        serde_json::json!({
            "method": "subscribe",
            "subscription": { "type": "allMids" }
        }),
    )];

    for dex in dexes.iter().filter(|dex| !dex.is_empty()) {
        subscriptions.push((
            format!("allMids:{dex}"),
            serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "allMids", "dex": dex }
            }),
        ));
    }

    let Some(address) = address else {
        return subscriptions;
    };

    for stream_type in ["allDexsClearinghouseState", "userFills", "spotState"] {
        subscriptions.push((
            format!("{stream_type}:{address}"),
            serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": stream_type, "user": address }
            }),
        ));
    }

    if dexes.iter().any(|dex| dex.is_empty()) {
        subscriptions.push((
            format!("openOrders:{address}"),
            serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "openOrders", "user": address }
            }),
        ));
    }

    for dex in dexes.iter().filter(|dex| !dex.is_empty()) {
        subscriptions.push((
            format!("openOrders:{address}:{dex}"),
            serde_json::json!({
                "method": "subscribe",
                "subscription": { "type": "openOrders", "user": address, "dex": dex }
            }),
        ));
    }

    subscriptions
}
