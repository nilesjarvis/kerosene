use super::{ActivityEntry, ActivityKind, Provider, record, safe_operation};
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;

#[derive(Default, Deserialize)]
struct FrameLabel<'a> {
    #[serde(borrow)]
    channel: Option<&'a str>,
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    #[serde(borrow)]
    method: Option<&'a str>,
    #[serde(borrow)]
    subscription: Option<SubscriptionLabel<'a>>,
}

#[derive(Deserialize)]
struct SubscriptionLabel<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
}

fn text_operation(text: &str) -> &'static str {
    let label = serde_json::from_str::<FrameLabel<'_>>(text).unwrap_or_default();
    label
        .subscription
        .and_then(|subscription| subscription.kind)
        .or(label.channel)
        .or(label.kind)
        .or(label.method)
        .map(safe_operation)
        .unwrap_or("other")
}

pub(crate) fn record_ws_frame(provider: Provider, message: &Message, received: bool) {
    let operation = match message {
        Message::Text(text) => text_operation(text),
        Message::Binary(_) => "binary",
        Message::Ping(_) => "ping",
        Message::Pong(_) => "pong",
        Message::Close(_) => "close",
        Message::Frame(_) => "frame",
    };
    let mut entry = ActivityEntry::new(
        provider,
        if received && operation == "error" {
            ActivityKind::WsReceiveError
        } else if received {
            ActivityKind::WsReceive
        } else {
            ActivityKind::WsSend
        },
        operation,
    );
    entry.bytes = Some(message.len() as u64);
    record(entry);
}

pub(crate) fn record_ws_lifecycle(provider: Provider, kind: ActivityKind) {
    record(ActivityEntry::new(provider, kind, "connection"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_allowlisted_channel_names_are_retained() {
        assert_eq!(
            text_operation(r#"{"channel":"l2Book","data":{"user":"PRIVATE"}}"#),
            "l2Book"
        );
        assert_eq!(
            text_operation(r#"{"type":"liquidations","cursor":"PRIVATE"}"#),
            "liquidations"
        );
        assert_eq!(
            text_operation(r#"{"channel":"PRIVATE","data":{}}"#),
            "other"
        );
        assert_eq!(text_operation("PRIVATE invalid JSON"), "other");
        assert_eq!(
            text_operation(
                r#"{"method":"subscribe","subscription":{"type":"userFills","user":"PRIVATE"}}"#
            ),
            "userFills"
        );
    }
}
