use super::super::{HYDROMANCER_RECONNECT_DELAY_SECS, HydromancerWsMessage};
use serde_json::Value;

pub(in crate::ws::hydromancer) fn hydromancer_control_message(
    msg_type: &str,
    data: &Value,
) -> Option<HydromancerWsMessage> {
    match msg_type {
        "connecting" => {
            if data
                .get("resuming")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                Some(HydromancerWsMessage::Resuming)
            } else {
                Some(HydromancerWsMessage::Connecting)
            }
        }
        "connected" => Some(HydromancerWsMessage::Connected),
        "reconnected" => Some(HydromancerWsMessage::Reconnected),
        "heartbeat" | "ping" => Some(HydromancerWsMessage::Heartbeat),
        "reconnecting" => {
            let error = data
                .get("error")
                .or_else(|| data.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("stream interrupted")
                .to_string();
            let retry_delay_secs = data
                .get("retryDelaySecs")
                .and_then(|v| v.as_u64())
                .unwrap_or(HYDROMANCER_RECONNECT_DELAY_SECS);
            Some(HydromancerWsMessage::Reconnecting {
                error: hydromancer_status_error(&error),
                retry_delay_secs,
            })
        }
        "error" | "disconnected" => {
            let error = data
                .get("error")
                .or_else(|| data.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("stream disconnected")
                .to_string();
            Some(HydromancerWsMessage::Disconnected(
                hydromancer_status_error(&error),
            ))
        }
        _ => None,
    }
}

fn hydromancer_status_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("unauthorized")
        || lower.contains("unauthenticated")
        || lower.contains("forbidden")
        || lower.contains("authentication")
        || lower.contains("authorization")
        || lower.contains("invalid api key")
        || lower.contains("invalid token")
        || lower.contains("api key")
        || lower.contains("token")
        || lower.contains("http 401")
        || lower.contains("http 403")
    {
        return "Hydromancer authentication failed. Check the API key in Settings > Integrations."
            .to_string();
    }

    if lower.contains("timeout") || lower.contains("timed out") {
        return format!("Hydromancer network timeout: {error}");
    }

    error.to_string()
}
