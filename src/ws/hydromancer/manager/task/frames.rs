use serde_json::Value;
use std::fmt;
use zeroize::Zeroizing;

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

#[derive(Clone)]
pub(super) struct HydromancerTextFrame {
    pub(super) json: Value,
    pub(super) kind: HydromancerTextFrameKind,
    pub(super) cursor: Option<Zeroizing<String>>,
    pub(super) session_id: Option<Zeroizing<String>>,
}

impl fmt::Debug for HydromancerTextFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HydromancerTextFrame")
            .field("kind", &self.kind)
            .field("has_cursor", &self.cursor.is_some())
            .field("has_session_id", &self.session_id.is_some())
            .finish()
    }
}

pub(super) fn parse_hydromancer_text_frame(text: &str) -> Option<HydromancerTextFrame> {
    let mut json = serde_json::from_str::<Value>(text).ok()?;
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
        .map(|value| Zeroizing::new(value.to_string()));
    let session_id = json
        .get("sessionId")
        .and_then(|value| value.as_str())
        .map(|value| Zeroizing::new(value.to_string()));
    if let Value::Object(fields) = &mut json {
        fields.remove("cursor");
        fields.remove("sessionId");
    }

    Some(HydromancerTextFrame {
        json,
        kind,
        cursor,
        session_id,
    })
}
