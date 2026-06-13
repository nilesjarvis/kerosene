use super::super::HydromancerRoutedMessage;

use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use tokio::sync::broadcast;
use zeroize::Zeroizing;

pub(super) struct HydromancerConnectUrl(Zeroizing<String>);

impl HydromancerConnectUrl {
    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for HydromancerConnectUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HydromancerConnectUrl(<redacted>)")
    }
}

pub(super) fn hydromancer_connect_url(
    api_key: &str,
    session_id: Option<&str>,
    last_cursor: Option<&str>,
) -> HydromancerConnectUrl {
    let mut url = Zeroizing::new(String::from("wss://api.hydromancer.xyz/ws?token="));
    url.push_str(api_key);
    if let Some(session_id) = session_id {
        url.push_str("&sessionId=");
        url.push_str(session_id);
        if let Some(cursor) = last_cursor {
            url.push_str("&cursor=");
            url.push_str(cursor);
        }
    }
    HydromancerConnectUrl(url)
}

pub(super) fn redact_hydromancer_error(error: impl ToString, api_key: &str) -> String {
    let mut message = Zeroizing::new(error.to_string());
    let api_key = api_key.trim();
    if !api_key.is_empty() {
        message = Zeroizing::new(message.replace(api_key, "<redacted>"));
    }
    redact_sensitive_query_values(message.as_str()).to_string()
}

fn redact_sensitive_query_values(message: &str) -> Zeroizing<String> {
    let mut redacted = Zeroizing::new(message.to_string());
    for key in ["token", "sessionId", "cursor"] {
        redacted = Zeroizing::new(redact_query_value(redacted.as_str(), key));
    }
    redacted
}

fn redact_query_value(message: &str, key: &str) -> String {
    let mut redacted = String::with_capacity(message.len());
    let mut remaining = message;
    let prefix = format!("{key}=");
    let prefix_lower = prefix.to_ascii_lowercase();

    while let Some(index) = remaining.to_ascii_lowercase().find(&prefix_lower) {
        let (before_value, value_and_rest) = remaining.split_at(index);
        redacted.push_str(before_value);
        redacted.push_str(&value_and_rest[..prefix.len()]);
        redacted.push_str("<redacted>");

        let value_start = prefix.len();
        let value = &value_and_rest[value_start..];
        let value_end = value
            .find(['&', ' ', '\t', '\n', '\r', '"', '\''])
            .unwrap_or(value.len());
        remaining = &value[value_end..];
    }

    redacted.push_str(remaining);
    redacted
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
        .or_else(|| json.get("channel"))
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

#[cfg(test)]
mod tests {
    use super::{hydromancer_connect_url, redact_hydromancer_error};

    #[test]
    fn hydromancer_connect_url_preserves_wire_format_but_redacts_debug() {
        let url = hydromancer_connect_url(
            "hydro-secret",
            Some("session-secret"),
            Some("cursor-secret"),
        );

        assert_eq!(
            url.as_str(),
            "wss://api.hydromancer.xyz/ws?token=hydro-secret&sessionId=session-secret&cursor=cursor-secret"
        );

        let rendered = format!("{url:?}");
        assert!(rendered.contains("<redacted>"));
        for secret in ["hydro-secret", "session-secret", "cursor-secret"] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn hydromancer_error_redaction_removes_token_query_and_raw_key() {
        let rendered = redact_hydromancer_error(
            "failed wss://api.hydromancer.xyz/ws?token=hydro-secret&sessionId=abc&cursor=def hydro-secret",
            "hydro-secret",
        );

        assert!(rendered.contains("token=<redacted>"));
        assert!(rendered.contains("sessionId=<redacted>"));
        assert!(rendered.contains("cursor=<redacted>"));
        assert!(!rendered.contains("hydro-secret"));
        assert!(!rendered.contains("abc"));
        assert!(!rendered.contains("def"));
    }

    #[test]
    fn hydromancer_error_redaction_handles_query_key_case_variants() {
        let rendered = redact_hydromancer_error(
            "failed wss://api.hydromancer.xyz/ws?Token=hydro-secret&sessionid=abc&CURSOR=def",
            "",
        );

        assert!(rendered.contains("Token=<redacted>"));
        assert!(rendered.contains("sessionid=<redacted>"));
        assert!(rendered.contains("CURSOR=<redacted>"));
        for secret in ["hydro-secret", "abc", "def"] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn hydromancer_error_redaction_keeps_normal_errors_useful() {
        let rendered = redact_hydromancer_error("HTTP 401 Unauthorized", "hydro-secret");

        assert_eq!(rendered, "HTTP 401 Unauthorized");
    }
}
