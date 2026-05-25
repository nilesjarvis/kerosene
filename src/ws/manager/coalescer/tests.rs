use super::*;
use serde_json::json;
use tokio::sync::broadcast;

mod flush;
mod latest_wins;
mod routing;
mod timing;

fn drain(rx: &mut broadcast::Receiver<WsRoutedMessage>) -> Vec<(String, Arc<Value>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push((msg.channel, msg.data));
    }
    out
}

fn next_due(sender: &CoalescedSender, reason: &str) -> Duration {
    match sender.next_due() {
        Some(duration) => duration,
        None => panic!("{reason}"),
    }
}
