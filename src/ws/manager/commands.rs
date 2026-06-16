use super::WsCommand;
use super::subscriptions::ActiveWsSubscriptions;

use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Outbound Command Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(super) struct WsCommandAction {
    pub(super) outbound_payload: Option<Value>,
    pub(super) disconnect_on_send_error: bool,
    pub(super) mark_ping_start: bool,
    pub(super) disconnect_after_handling: bool,
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
