use super::subscriptions::ActiveWsSubscriptions;
use super::{WsCommand, redacted_ws_value};

use serde_json::Value;
use std::fmt;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Outbound Command Planning
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(super) struct WsCommandAction {
    pub(super) outbound_payload: Option<Value>,
    pub(super) disconnect_on_send_error: bool,
    pub(super) mark_ping_start: bool,
    pub(super) disconnect_after_handling: bool,
}

impl fmt::Debug for WsCommandAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WsCommandAction")
            .field(
                "outbound_payload",
                &self.outbound_payload.as_ref().map(redacted_ws_value),
            )
            .field("disconnect_on_send_error", &self.disconnect_on_send_error)
            .field("mark_ping_start", &self.mark_ping_start)
            .field("disconnect_after_handling", &self.disconnect_after_handling)
            .finish()
    }
}

impl WsCommandAction {
    fn none() -> Self {
        Self {
            outbound_payload: None,
            disconnect_on_send_error: false,
            mark_ping_start: false,
            disconnect_after_handling: false,
        }
    }

    fn outbound(payload: Value, disconnect_on_send_error: bool) -> Self {
        Self {
            outbound_payload: Some(payload),
            disconnect_on_send_error,
            mark_ping_start: false,
            disconnect_after_handling: false,
        }
    }

    fn ping() -> Self {
        Self {
            outbound_payload: Some(serde_json::json!({ "method": "ping" })),
            disconnect_on_send_error: true,
            mark_ping_start: true,
            disconnect_after_handling: false,
        }
    }

    fn reconnect() -> Self {
        Self {
            outbound_payload: None,
            disconnect_on_send_error: false,
            mark_ping_start: false,
            disconnect_after_handling: true,
        }
    }
}

pub(super) fn handle_ws_command(
    active_subs: &mut ActiveWsSubscriptions,
    command: WsCommand,
) -> WsCommandAction {
    match command {
        WsCommand::Subscribe { topic, payload } => active_subs
            .subscribe(topic, payload)
            .map(|payload| WsCommandAction::outbound(payload, true))
            .unwrap_or_else(WsCommandAction::none),
        WsCommand::Unsubscribe { topic, payload } => active_subs
            .unsubscribe(topic, payload)
            .removed_payload()
            .map(|payload| WsCommandAction::outbound(payload, true))
            .unwrap_or_else(WsCommandAction::none),
        WsCommand::Ping => WsCommandAction::ping(),
        WsCommand::Reconnect => WsCommandAction::reconnect(),
    }
}
