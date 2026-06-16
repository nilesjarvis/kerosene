use super::frames::{HydromancerTextFrame, HydromancerTextFrameKind};

use serde_json::Value;
use std::fmt;
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Hydromancer Session State
// ---------------------------------------------------------------------------

#[derive(Default)]
pub(super) struct HydromancerSessionState {
    session_id: Option<Zeroizing<String>>,
    last_cursor: Option<Zeroizing<String>>,
    connection_ready: bool,
}

impl fmt::Debug for HydromancerSessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HydromancerSessionState")
            .field("has_session_id", &self.session_id.is_some())
            .field("has_last_cursor", &self.last_cursor.is_some())
            .field("connection_ready", &self.connection_ready)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HydromancerFrameAction {
    pub(super) resend_subscriptions: bool,
    pub(super) send_pong: bool,
}

impl HydromancerSessionState {
    pub(super) fn begin_connection(&mut self) {
        self.connection_ready = false;
    }

    pub(super) fn session_id(&self) -> Option<&str> {
        self.session_id
            .as_ref()
            .map(|session_id| session_id.as_str())
    }

    pub(super) fn last_cursor(&self) -> Option<&str> {
        self.last_cursor.as_ref().map(|cursor| cursor.as_str())
    }

    pub(super) fn connection_ready(&self) -> bool {
        self.connection_ready
    }

    pub(super) fn connecting_data(&self) -> Value {
        serde_json::json!({
            "resuming": self.session_id.is_some(),
            "hasCursor": self.last_cursor.is_some(),
        })
    }

    pub(super) fn apply_text_frame(
        &mut self,
        frame: &HydromancerTextFrame,
    ) -> HydromancerFrameAction {
        if let Some(cursor) = &frame.cursor {
            self.last_cursor = Some(cursor.clone());
        }

        match frame.kind {
            HydromancerTextFrameKind::Connected => {
                self.connection_ready = true;
                self.session_id = frame.session_id.clone();
                HydromancerFrameAction {
                    resend_subscriptions: true,
                    send_pong: false,
                }
            }
            HydromancerTextFrameKind::Reconnected => {
                self.connection_ready = true;
                self.session_id = frame.session_id.clone();
                HydromancerFrameAction {
                    resend_subscriptions: true,
                    send_pong: false,
                }
            }
            HydromancerTextFrameKind::Ping => HydromancerFrameAction {
                resend_subscriptions: false,
                send_pong: true,
            },
            HydromancerTextFrameKind::Other => HydromancerFrameAction {
                resend_subscriptions: false,
                send_pong: false,
            },
        }
    }
}
