use super::super::HydromancerRoutedMessage;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

pub(super) fn hydromancer_connect_url(
    api_key: &str,
    session_id: Option<&str>,
    last_cursor: Option<&str>,
) -> String {
    let mut url = format!("wss://api.hydromancer.xyz/ws?token={api_key}");
    if let Some(session_id) = session_id {
        url.push_str("&sessionId=");
        url.push_str(session_id);
        if let Some(cursor) = last_cursor {
            url.push_str("&cursor=");
            url.push_str(cursor);
        }
    }
    url
}

pub(super) fn hydromancer_unsubscribe_payload(payload: &Value) -> Value {
    let Some(subscription) = payload.get("subscription") else {
        return serde_json::json!({ "type": "unsubscribe" });
    };

    serde_json::json!({
        "type": "unsubscribe",
        "subscription": subscription,
    })
}

pub(super) fn broadcast_hydromancer_control(
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
    msg_type: &str,
    data: Value,
) -> Result<usize, broadcast::error::SendError<HydromancerRoutedMessage>> {
    msg_tx.send(HydromancerRoutedMessage {
        msg_type: msg_type.to_string(),
        data: Arc::new(data),
    })
}

pub(super) fn broadcast_hydromancer_json(
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
    json: Value,
) -> Result<usize, broadcast::error::SendError<HydromancerRoutedMessage>> {
    let msg_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    msg_tx.send(HydromancerRoutedMessage {
        msg_type,
        data: Arc::new(json),
    })
}

pub(super) fn broadcast_hydromancer_reconnecting(
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
    error: impl ToString,
    retry_delay_secs: u64,
) -> Result<usize, broadcast::error::SendError<HydromancerRoutedMessage>> {
    broadcast_hydromancer_control(
        msg_tx,
        "reconnecting",
        serde_json::json!({
            "error": error.to_string(),
            "retryDelaySecs": retry_delay_secs,
        }),
    )
}

pub(super) fn broadcast_hydromancer_heartbeat(
    msg_tx: &broadcast::Sender<HydromancerRoutedMessage>,
) -> Result<usize, broadcast::error::SendError<HydromancerRoutedMessage>> {
    broadcast_hydromancer_control(msg_tx, "heartbeat", serde_json::json!({}))
}
