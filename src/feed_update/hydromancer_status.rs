use crate::app_time::now_ms;
use crate::helpers::redact_sensitive_response_text;
use crate::ws::HydromancerWsMessage;

pub(super) fn apply_hydromancer_control_message(
    message: &HydromancerWsMessage,
    status: &mut String,
    last_rx_ms: &mut Option<u64>,
) {
    match message {
        HydromancerWsMessage::Connecting => {
            *status = "Connecting".to_string();
        }
        HydromancerWsMessage::Resuming => {
            *status = "Resuming session".to_string();
        }
        HydromancerWsMessage::Connected => {
            *last_rx_ms = Some(now_ms());
            *status = "Connected".to_string();
        }
        HydromancerWsMessage::Reconnected => {
            *last_rx_ms = Some(now_ms());
            *status = "Reconnected".to_string();
        }
        HydromancerWsMessage::Heartbeat => {
            *last_rx_ms = Some(now_ms());
        }
        HydromancerWsMessage::Reconnecting {
            error,
            retry_delay_secs,
        } => {
            let error = redact_sensitive_response_text(error);
            *status = format!("Reconnecting in {retry_delay_secs}s: {error}");
        }
        HydromancerWsMessage::Disconnected(error) => {
            *last_rx_ms = None;
            let error = redact_sensitive_response_text(error);
            *status = format!("Disconnected: {error}");
        }
        HydromancerWsMessage::Lagged { skipped } => {
            *last_rx_ms = None;
            *status = format!("Stream lagged; reconnecting after skipping {skipped} messages");
        }
        HydromancerWsMessage::Event(_) | HydromancerWsMessage::TrackedTrade(_) => {}
    }
}
