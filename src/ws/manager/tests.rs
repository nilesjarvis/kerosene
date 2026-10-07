use super::*;
use serde_json::json;
use std::time::Duration;

mod reconnect;
mod stale_read;
mod timeout;

#[tokio::test]
async fn book_manager_shares_equal_precision_and_isolates_unattributed_frames() {
    let (fine, _) = get_book_manager((Some(5), None));
    let (same, _) = get_book_manager((Some(5), None));
    let (coarse, _) = get_book_manager((Some(3), None));
    let (mantissa, _) = get_book_manager((Some(5), Some(5)));
    let (general, _) = get_manager();
    assert!(fine.inner.same_channel(&same.inner));
    assert!(!fine.inner.same_channel(&coarse.inner));
    assert!(!fine.inner.same_channel(&mantissa.inner));
    assert!(!fine.inner.same_channel(&general.inner));
}

const DEBUG_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

#[test]
fn ws_command_debug_redacts_user_subscription_payload() {
    let command = WsCommand::Subscribe {
        topic: format!("userFills:{DEBUG_ADDRESS}"),
        payload: json!({
            "method": "subscribe",
            "subscription": {
                "type": "userFills",
                "user": DEBUG_ADDRESS,
                "token": "payload-token"
            }
        }),
    };

    let rendered = format!("{command:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(rendered.contains("subscription_type: Some(\"userFills\")"));
    assert!(!rendered.contains(DEBUG_ADDRESS));
    assert!(!rendered.contains("payload-token"));
}

#[test]
fn ws_routed_message_debug_redacts_raw_data() {
    let message = WsRoutedMessage {
        channel: "userFills".to_string(),
        data: Arc::new(json!({
            "user": DEBUG_ADDRESS,
            "fills": [{ "hash": "fill-secret" }]
        })),
    };

    let rendered = format!("{message:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(DEBUG_ADDRESS));
    assert!(!rendered.contains("fill-secret"));
}

struct PendingWriteSink;

impl futures::Sink<WsMsg> for PendingWriteSink {
    type Error = ();

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Pending
    }

    fn start_send(self: std::pin::Pin<&mut Self>, _item: WsMsg) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Pending
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Pending
    }
}

#[tokio::test]
async fn ws_text_send_times_out_for_pending_sink() {
    let mut sink = PendingWriteSink;

    assert!(!send_ws_text_with_timeout(&mut sink, "{}".to_string()).await);
}
