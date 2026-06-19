use super::{redacted_ws_topic_debug_value, redacted_ws_value};

use serde_json::Value;
use std::fmt;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Inbound Frame Parsing
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(super) enum WsTextFrame {
    Pong,
    Data { channel: String, data: Value },
    Ignored,
}

impl fmt::Debug for WsTextFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pong => f.write_str("Pong"),
            Self::Data { channel, data } => f
                .debug_struct("Data")
                .field("channel", &redacted_ws_topic_debug_value(channel))
                .field("data", &redacted_ws_value(data))
                .finish(),
            Self::Ignored => f.write_str("Ignored"),
        }
    }
}

pub(super) fn parse_ws_text_frame(text: &str) -> WsTextFrame {
    let Ok(json) = serde_json::from_str::<Value>(text) else {
        return WsTextFrame::Ignored;
    };
    let Some(channel) = json.get("channel").and_then(|value| value.as_str()) else {
        return WsTextFrame::Ignored;
    };

    if channel == "pong" {
        return WsTextFrame::Pong;
    }

    let Some(data) = json.get("data") else {
        return WsTextFrame::Ignored;
    };

    WsTextFrame::Data {
        channel: channel.to_string(),
        data: data.clone(),
    }
}
