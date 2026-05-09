mod analytics;
mod app;
mod feeds;
mod input;
mod wallet;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Timer And Input Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        self.push_keyboard_subscriptions(subs);
        self.push_ui_timer_subscriptions(subs);
        self.push_account_timer_subscriptions(subs);
        self.push_hyperdash_timer_subscriptions(subs);
    }

    pub(super) fn push_post_window_timer_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        self.push_wallet_tracker_timer_subscriptions(subs);
        self.push_analytics_timer_subscriptions(subs);
    }
}
