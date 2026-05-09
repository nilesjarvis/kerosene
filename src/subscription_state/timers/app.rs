use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Subscription;

// ---------------------------------------------------------------------------
// App UI Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_ui_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if !self.toasts.is_empty() || self.nuke_confirmation.is_some() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(1))
                    .map(|_| Message::TickToastCleanup),
            );
        }

        if self.has_loading_activity() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(40))
                    .map(|_| Message::SpinnerTick),
            );
        }

        subs.push(
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::StatusBarTick),
        );

        subs.push(
            iced::time::every(std::time::Duration::from_secs(60 * 15)).map(|_| Message::Tick),
        );

        if !self.hydromancer_api_key.trim().is_empty()
            && self
                .charts
                .values()
                .any(|instance| instance.macro_indicators.show_funding_rate)
        {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60 * 5))
                    .map(|_| Message::FundingRefreshTick),
            );
        }
    }
}
