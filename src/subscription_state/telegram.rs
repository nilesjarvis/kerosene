use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::telegram_fast_feed::{
    TelegramFastFeedStreamParams, bundled_telegram_api_id, telegram_fast_feed_stream,
};
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_telegram_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if !self.telegram_feed.fast_mode_enabled
            || !self.pane_is_open(|kind| matches!(kind, PaneKind::TelegramFeed))
        {
            return;
        }

        let Some(api_id) = self
            .telegram_feed
            .fast_api_id
            .or_else(bundled_telegram_api_id)
        else {
            return;
        };
        if self.telegram_feed.channels.is_empty() && self.telegram_feed.private_channels.is_empty()
        {
            return;
        }

        let fast_reconnect_nonce = self.telegram_feed.fast_reconnect_nonce;
        subs.push(
            Subscription::run_with(
                TelegramFastFeedStreamParams {
                    api_id,
                    channels: self.telegram_feed.channels.clone(),
                    private_channels: self.telegram_feed.private_channels.clone(),
                    reconnect_nonce: fast_reconnect_nonce,
                },
                telegram_fast_feed_stream,
            )
            .map(move |event| Message::TelegramFastFeedEvent(fast_reconnect_nonce, event)),
        );
    }
}
