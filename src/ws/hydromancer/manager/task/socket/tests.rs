use super::*;

use futures::channel::mpsc;
use futures::{FutureExt as _, StreamExt as _, executor::block_on};
use serde_json::json;

mod commands;
mod frames;

fn text_msg(msg: WsMsg) -> String {
    match msg {
        WsMsg::Text(text) => text.to_string(),
        other => panic!("expected text message, got {other:?}"),
    }
}

fn next_text_msg_or_panic(sent: &mut mpsc::UnboundedReceiver<WsMsg>) -> String {
    match block_on(sent.next()) {
        Some(msg) => text_msg(msg),
        None => panic!("missing websocket message"),
    }
}

fn routed_msg_or_panic(
    receiver: &mut broadcast::Receiver<HydromancerRoutedMessage>,
) -> HydromancerRoutedMessage {
    match receiver.try_recv() {
        Ok(message) => message,
        Err(error) => panic!("missing routed frame: {error}"),
    }
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
async fn hydromancer_text_send_times_out_for_pending_sink() {
    let mut sink = PendingWriteSink;

    assert!(!send_text(&mut sink, "{}".to_string()).await);
}
