use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::x_feed_stream::{XFeedStreamParams, x_feed_stream};
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_x_feed_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if !self.x_feed.streaming_enabled
            || !self.pane_is_open(|kind| matches!(kind, PaneKind::XFeed))
            || self.x_feed.bearer_token.trim().is_empty()
            || self.x_feed.handles.is_empty()
        {
            return;
        }

        subs.push(
            Subscription::run_with(
                XFeedStreamParams {
                    bearer_token: self.x_feed.bearer_token.trim().to_string(),
                    handles: self.x_feed.handles.clone(),
                    reconnect_nonce: self.x_feed.stream_reconnect_nonce,
                },
                x_feed_stream,
            )
            .map(Message::XFeedStreamEvent),
        );
    }
}
