use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Hydromancer Text Frame Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HydromancerTextFrameKind {
    Connected,
    Reconnected,
    Ping,
    Other,
}

#[derive(Debug, Clone)]
pub(super) struct HydromancerTextFrame {
    pub(super) json: Value,
    pub(super) kind: HydromancerTextFrameKind,
    pub(super) cursor: Option<String>,
    pub(super) session_id: Option<String>,
}

pub(super) fn parse_hydromancer_text_frame(text: &str) -> Option<HydromancerTextFrame> {
    let json = serde_json::from_str::<Value>(text).ok()?;
    let msg_type = json.get("type").and_then(|value| value.as_str())?;
    let kind = match msg_type {
        "connected" => HydromancerTextFrameKind::Connected,
        "reconnected" => HydromancerTextFrameKind::Reconnected,
        "ping" => HydromancerTextFrameKind::Ping,
        _ => HydromancerTextFrameKind::Other,
    };
    let cursor = json
        .get("cursor")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let session_id = json
        .get("sessionId")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    Some(HydromancerTextFrame {
        json,
        kind,
        cursor,
        session_id,
    })
}
