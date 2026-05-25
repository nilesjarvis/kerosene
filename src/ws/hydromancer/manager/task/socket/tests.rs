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
