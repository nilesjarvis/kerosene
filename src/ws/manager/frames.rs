use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Inbound Frame Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(super) enum WsTextFrame {
    Pong,
    Data { channel: String, data: Value },
    Ignored,
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
